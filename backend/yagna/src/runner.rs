#![allow(unused)]

use actix::prelude::*;
use actix_http::httpmessage::HttpMessage;
use awc::error::WsClientError;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Timelike, Utc};
use futures::channel::oneshot;
use futures::prelude::*;
use futures::TryFutureExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha3::digest::Digest;
use std::collections::HashSet;
use std::convert::TryInto;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::AcqRel;
use std::sync::Arc;
use std::time::Duration;
use ya_client::market::MarketRequestorApi;
use ya_client::model;
use ya_client::model::market::{
    proposal::State as ProposalState, AgreementProposal, Demand, Proposal, RequestorEvent,
};
use ya_client::web::WebClient;
use zip::CompressionMethod;

use super::negotiator::*;
use super::storage::{DistSlot, DistStorage};
use crate::YagnaEngine;
use gwr_backend::{dispatcher::TaskDef, rt::Engine, run_local_code, WorkDir};

async fn push_image(
    hub_url: Arc<str>,
    image: Vec<u8>,
) -> Result<String, awc::error::WsClientError> {
    let c = awc::Client::new();

    let hex = format!("{:x}", <sha3::Sha3_224 as Digest>::digest(image.as_slice()));
    let download_url = format!("{}app-{}.yimg", hub_url, &hex[0..8]);
    let upload_url = format!("{}upload/app-{}.yimg", hub_url, &hex[0..8]);
    let response = c
        .put(&upload_url)
        .content_length(image.len() as u64)
        .content_type("application/octet-stream")
        .send_body(image)
        .await?;
    if response.status().is_success() {
        Ok(format!("hash:sha3:{}:{}", hex, download_url))
    } else {
        Err(WsClientError::InvalidResponseStatus(response.status()))
    }
}

struct PaymentManager {
    payment_api: ya_client::payment::requestor::PaymentRequestorApi,
    allocation_id: String,
    total_amount: BigDecimal,
    amount_paid: BigDecimal,
    valid_agreements: HashSet<String>,
    last_debit_note_event: DateTime<Utc>,
    last_invoice_event: DateTime<Utc>,
}

impl Actor for PaymentManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.update_debit_notes(ctx);
        self.update_invoices(ctx);
    }
}

impl PaymentManager {
    fn update_debit_notes(&mut self, ctx: &mut <PaymentManager as Actor>::Context) {
        let mut ts = self.last_debit_note_event;
        let api = self.payment_api.clone();

        let f = async move {
            let events = api.get_debit_note_events(Some(&ts), None).await?;
            for event in events {
                log::debug!("got debit note: {:?}", event);
                ts = event.timestamp;
            }
            Ok::<_, anyhow::Error>(ts)
        }
        .into_actor(self)
        .then(|ts, this, ctx: &mut Context<Self>| {
            match ts {
                Ok(ts) => this.last_debit_note_event = ts,
                Err(e) => {
                    log::error!("debit note event error: {}", e);
                }
            }
            ctx.run_later(Duration::from_secs(10), |this, ctx| {
                this.update_debit_notes(ctx)
            });
            fut::ready(())
        });

        let _ = ctx.spawn(f);
    }

    fn update_invoices(&mut self, ctx: &mut <PaymentManager as Actor>::Context) {
        let mut ts = self.last_invoice_event;
        let api = self.payment_api.clone();

        let f = async move {
            let events = api.get_invoice_events(Some(&ts), None).await?;
            let mut new_invoices = Vec::new();
            for event in events {
                log::debug!("Got invoice: {:?}", event);
                if event.event_type == model::payment::EventType::Received {
                    let invoice = api.get_invoice(&event.invoice_id).await?;
                    new_invoices.push(invoice);
                }
                ts = event.timestamp;
            }
            Ok::<_, anyhow::Error>((ts, new_invoices))
        }
        .into_actor(self)
        .then(
            |result: Result<(_, Vec<model::payment::Invoice>), _>,
             this,
             ctx: &mut Context<Self>| {
                match result {
                    Ok((ts, invoices)) => {
                        this.last_invoice_event = ts;
                        for invoice in invoices {
                            let api = this.payment_api.clone();

                            if this.valid_agreements.remove(&invoice.agreement_id) {
                                let invoice_id = invoice.invoice_id;
                                log::info!(
                                    "Accepting invoice amounted {} GNT, issuer: {}",
                                    invoice.amount,
                                    invoice.issuer_id
                                );
                                this.amount_paid += invoice.amount.clone();
                                let acceptance = model::payment::Acceptance {
                                    total_amount_accepted: invoice.amount.clone(),
                                    allocation_id: this.allocation_id.clone(),
                                };
                                Arbiter::spawn(async move {
                                    if let Err(e) =
                                        api.accept_invoice(&invoice_id, &acceptance).await
                                    {
                                        log::error!("invoice {} accept error: {}", invoice_id, e)
                                    }
                                });
                            } else {
                                let invoice_id = invoice.invoice_id;

                                let spec = model::payment::Rejection {
                                    rejection_reason:
                                        model::payment::RejectionReason::UnsolicitedService,
                                    total_amount_accepted: 0.into(),
                                    message: Some("invoice received before results".to_string()),
                                };
                                Arbiter::spawn(async move {
                                    if let Err(e) = api.reject_invoice(&invoice_id, &spec).await {
                                        log::error!("invoice: {} reject error: {}", invoice_id, e);
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("invoice processing error: {}", e);
                    }
                }
                ctx.run_later(Duration::from_secs(10), |this, ctx| {
                    this.update_invoices(ctx)
                });
                fut::ready(())
            },
        );

        let _ = ctx.spawn(f);
    }
}

struct AcceptAgreement {
    agreement_id: String,
}

impl Message for AcceptAgreement {
    type Result = anyhow::Result<()>;
}

impl Handler<AcceptAgreement> for PaymentManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: AcceptAgreement, ctx: &mut Self::Context) -> Self::Result {
        self.valid_agreements.insert(msg.agreement_id);
        Ok(())
    }
}

struct GetPending;

impl Message for GetPending {
    type Result = usize;
}

impl Handler<GetPending> for PaymentManager {
    type Result = MessageResult<GetPending>;

    fn handle(&mut self, msg: GetPending, ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.valid_agreements.len())
    }
}

struct ReleaseAllocation;

impl Message for ReleaseAllocation {
    type Result = anyhow::Result<()>;
}

impl Handler<ReleaseAllocation> for PaymentManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: ReleaseAllocation, ctx: &mut Self::Context) -> Self::Result {
        let api = self.payment_api.clone();
        let allocation_id = self.allocation_id.clone();
        let _ = ctx.spawn(
            async move {
                log::info!("Releasing allocation");
                api.release_allocation(&allocation_id).await;
            }
            .into_actor(self),
        );
        Ok(())
    }
}

async fn allocate_funds_for_task(
    payment_api: &ya_client::payment::requestor::PaymentRequestorApi,
    n_tasks: usize,
) -> anyhow::Result<Addr<PaymentManager>> {
    let now = Utc::now();
    let total_amount: BigDecimal = ((n_tasks * 8) as u64).into();
    let new_allocation = model::payment::NewAllocation {
        //address: None,
        //payment_platform: None,
        total_amount: total_amount.clone(),
        timeout: None,
        make_deposit: false,
    };
    let allocation = payment_api.create_allocation(&new_allocation).await?;
    log::info!("Allocated {} GNT.", &allocation.total_amount);

    let manager = PaymentManager {
        payment_api: payment_api.clone(),
        allocation_id: allocation.allocation_id,
        total_amount,
        amount_paid: 0.into(),
        valid_agreements: Default::default(),
        last_debit_note_event: now,
        last_invoice_event: now,
    };
    Ok(manager.start())
}

#[derive(Debug)]
struct TaskResult {
    agreement_id: String,
    task_def: TaskDef,
}

async fn process_task(
    storage: DistStorage,
    client: WebClient,
    p: Addr<PaymentManager>,
    a: Addr<AgreementProducer>,
    output_path: PathBuf,
    merge_path: PathBuf,
    task: TaskDef,
) -> anyhow::Result<TaskResult> {
    let mut commands = Vec::new();

    commands.push(serde_json::json!({"deploy": { }}));
    commands.push(serde_json::json!({"start": { "args": [] }}));

    let input_path: PathBuf = "/in".into();
    for blob_path in task.blobs() {
        let file_name = storage.upload_file(&output_path.join(blob_path)).await?;
        commands.push(serde_json::json!({"transfer": {
            "from": file_name,
            "to": format!("container:/in/{}", blob_path)
        }}));
    }
    let task_file = storage.upload_json(&task).await?;
    commands.push(serde_json::json!({"transfer": {
        "from": task_file,
        "to": "container:/in/task.json"
    }}));

    commands.push(serde_json::json!({"run": {
      "entry_point": "main",
      "args": ["exec", "/in/task.json", "/out/task.json"]
    }}));
    let mut outputs = Vec::new();
    for blob_path in task.outputs() {
        log::debug!("output blob filename={}", blob_path);
        let slot = storage.download_slot().await?;
        commands.push(serde_json::json!({"transfer": {
            "from": format!("container:/out/{}", blob_path),
            "to": slot.url()
        }}));
        outputs.push((slot, merge_path.join(blob_path)))
    }
    let output_slot = storage.download_slot().await?;
    commands.push(serde_json::json!({"transfer": {
        "from": "container:/out/task.json",
        "to": output_slot.url()
    }}));

    let commands_cnt = commands.len();
    let script_text = serde_json::to_string_pretty(&commands)?;
    log::trace!("script=[{}]", script_text);
    let script = ya_client::model::activity::ExeScriptRequest::new(script_text);

    loop {
        match try_process_task(
            commands_cnt,
            &script,
            &output_slot,
            &outputs,
            client.clone(),
            p.clone(),
            a.clone(),
            output_path.clone(),
            merge_path.clone(),
            task.clone(),
        )
        .await
        {
            Ok(v) => return Ok(v),
            Err(e) => {
                log::error!("fail to process subtask: {}", e);
                log::info!("retry");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn try_process_task(
    commands_cnt: usize,
    script: &ya_client::model::activity::ExeScriptRequest,
    output_slot: &DistSlot,
    outputs: &[(DistSlot, PathBuf)],
    client: WebClient,
    p: Addr<PaymentManager>,
    a: Addr<AgreementProducer>,
    output_path: PathBuf,
    merge_path: PathBuf,
    task: TaskDef,
) -> anyhow::Result<TaskResult> {
    let activity_api = client.interface::<ya_client::activity::ActivityRequestorApi>()?;
    let agreement_id = a.send(NewAgreement).await??;
    let activity_id = match activity_api.control().create_activity(&agreement_id).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("activity create error: {}", e);
            return Err(e.into());
        }
    };

    log::info!("Activity created. Sending ExeScript... [{}]", activity_id);
    let batch_id = activity_api
        .control()
        .exec(script.clone(), &activity_id)
        .await?;

    loop {
        let state = activity_api.state().get_state(&activity_id).await?;
        if !state.alive() {
            log::error!("activity {} is NOT ALIVE any more.", activity_id);
            break;
        }

        log::info!("activity {} state: {:?}", activity_id, state);
        let results = match activity_api
            .control()
            .get_exec_batch_results(&activity_id, &batch_id, Some(60.), None)
            .await
        {
            Ok(v) => v,
            Err(ya_client::Error::TimeoutError { .. }) => Vec::default(),
            Err(e) => return Err(e.into()),
        };

        log::debug!("ExeScript batch results: {:#?}", results);

        if results.len() >= commands_cnt {
            break;
        }

        tokio::time::delay_for(Duration::from_millis(700)).await;
    }

    // TODO: task output path resolve
    let task_def = output_slot.download_json().await?;

    let _err = p
        .send(AcceptAgreement {
            agreement_id: agreement_id.clone(),
        })
        .await;

    for (slot, output) in outputs {
        log::info!(
            "ExeScript finished. Downloading result...   [{}]",
            activity_id
        );
        log::debug!("Downloading: {}", output.display());
        slot.download(&output).await?;
    }
    if let Err(e) = activity_api.control().destroy_activity(&activity_id).await {
        log::error!("fail to destroy activity: {}", e);
    }

    log::info!("Task finished.   [{}]", activity_id);
    Ok(TaskResult {
        agreement_id,
        task_def,
    })
}

pub fn run(
    hub_addr: Option<String>,
    token: Option<String>,
    subnet: Option<String>,
    engine: impl YagnaEngine + 'static,
    wasm_path: &Path,
    timeout: Duration,
    args: &[String],
) -> anyhow::Result<()> {
    let _ = dotenv::dotenv().ok();
    let token = match token {
        Some(token) => token,
        None => std::env::var("YAGNA_APPKEY")?,
    };
    let client = ya_client::web::WebClient::with_token(&token);

    let mut sys = System::new("wasm-runner");
    let mut w = WorkDir::new("lwg")?;
    let image = engine.build_image(&wasm_path)?;
    log::info!("Locally splitting work into tasks");
    let output_path = w.split_output()?;
    {
        let mut split_args = Vec::new();
        split_args.push("split".to_owned());
        split_args.push("/task_dir/".to_owned());
        split_args.extend(args.iter().cloned());
        run_local_code(engine.clone(), wasm_path, &output_path, split_args)?;
    }

    let tasks_path = output_path.join("tasks.json");

    log::debug!("reading: {}", tasks_path.display());
    let tasks: Vec<TaskDef> =
        serde_json::from_reader(fs::OpenOptions::new().read(true).open(tasks_path)?)?;
    log::info!("Created {} tasks", tasks.len());

    let merge_path = w.merge_path()?;
    let output_file = merge_path.join("tasks.json");
    let merge_path_ref = merge_path.clone();

    let storage_server: Arc<str> = "http://3.249.139.167:8000/".into();
    let payment_api: ya_client::payment::requestor::PaymentRequestorApi = client.interface()?;
    let task_output_path = output_path;
    let merge_engine = engine.clone();
    sys.block_on(async move {
        // TODO: Catch error
        let image = push_image(storage_server.clone(), image).await.unwrap();
        log::info!("Binary image uploaded: {}", image);

        let node_name = "test1";
        let my_demand = engine.build_demand(node_name, &image, timeout, subnet.as_ref())?;
        let market_api: ya_client::market::MarketRequestorApi = client.interface()?;

        let storage = DistStorage::new(storage_server);
        let output_tasks = merge_path_ref.join("tasks.json");
        let payment_man = allocate_funds_for_task(&payment_api, tasks.len()).await?;

        let agreements = {
            let a = agreement_producer(&market_api, &my_demand).await?;
            let agreements = futures::future::join_all(tasks.into_iter().map(|t| {
                process_task(
                    storage.clone(),
                    client.clone(),
                    payment_man.clone(),
                    a.clone(),
                    task_output_path.clone(),
                    merge_path_ref.clone(),
                    t,
                )
            }))
            .await;
            let _ = a.send(Kill).await;
            agreements
        };

        let mut tasks = Vec::new();
        for res in agreements {
            let result = res?;
            tasks.push(result.task_def);
        }

        std::fs::write(output_tasks, serde_json::to_vec_pretty(&tasks)?)?;
        loop {
            let pending = payment_man.send(GetPending).await?;
            if pending == 0 {
                break;
            }
            log::warn!("still {} pending payments", pending);
            tokio::time::delay_for(Duration::from_millis(700)).await;
        }
        payment_man.send(ReleaseAllocation).await?;
        log::info!("Work done and paid. Enjoy results.");

        Ok::<_, anyhow::Error>(())
    })?;

    {
        let mut merge_args = Vec::new();
        merge_args.push("merge".to_owned());
        merge_args.push("/task_dir/split/tasks.json".to_owned());
        merge_args.push("/task_dir/merge/tasks.json".to_owned());
        merge_args.push("--".to_owned());
        merge_args.extend(args.iter().cloned());
        run_local_code(
            merge_engine,
            wasm_path,
            merge_path.parent().unwrap(),
            merge_args,
        )?;
    }

    Ok(())
}

use crate::local_runner::run_local_code;
use crate::wasm_engine::Engine;
use crate::workdir::WorkDir;
use actix::prelude::*;
use actix_http::httpmessage::HttpMessage;
use awc::error::WsClientError;
use chrono::Utc;
use futures::channel::oneshot;
use futures::prelude::*;
use futures::TryFutureExt;
use gwasm_dispatcher::TaskDef;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha3::digest::Digest;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use wasmtime::wasm::wasm_engine_delete;
use ya_client::market::MarketRequestorApi;
use ya_client::model;
use ya_client::model::market::{
    proposal::State as ProposalState, AgreementProposal, Demand, Proposal, RequestorEvent,
};
use ya_client::web::WebClient;
use zip::CompressionMethod;

#[derive(Clone)]
struct DistStorage {
    url: Arc<str>,
}

struct DistSlot {
    upload_url: String,
    download_url: String,
}

impl DistSlot {
    fn url(&self) -> &str {
        self.upload_url.as_str()
    }

    async fn download(&self, out_path: &Path) -> anyhow::Result<()> {
        let c = awc::Client::new();

        let mut response = c
            .get(&self.download_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?;

        let payload = response.take_payload();
        let mut fs = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out_path)?;
        Ok(payload
            .for_each(|b| {
                let bytes = b.unwrap();
                fs.write_all(bytes.as_ref()).unwrap();
                future::ready(())
            })
            .await)
    }

    async fn download_json<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let c = awc::Client::new();
        let b = c
            .get(&self.download_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?
            .body()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?;

        Ok(serde_json::from_slice(b.as_ref())?)
    }
}

impl DistStorage {
    fn new(storage_url: Arc<str>) -> Self {
        let url = storage_url;
        Self { url }
    }

    async fn upload_bytes(&self, prefix: &str, bytes: Vec<u8>) -> anyhow::Result<String> {
        let c = awc::Client::new();
        let id = uuid::Uuid::new_v4();
        let upload_url = format!("{}upload/{}-{}", self.url, prefix, id);

        let response = c
            .put(&upload_url)
            .content_length(bytes.len() as u64)
            .content_type("application/octet-stream")
            .send_body(bytes)
            .await
            .map_err(|e| anyhow::anyhow!("upload bytes: {}", e))?;

        Ok(format!("{}{}-{}", self.url, prefix, id))
    }

    async fn upload_file(&self, path: &Path) -> anyhow::Result<String> {
        self.upload_bytes("blob", std::fs::read(path)?).await
    }

    async fn upload_json<T: Serialize>(&self, obj: &T) -> anyhow::Result<String> {
        let bytes = serde_json::to_vec_pretty(obj)?;
        self.upload_bytes("json", bytes).await
    }

    async fn download_slot(&self) -> anyhow::Result<DistSlot> {
        let id = uuid::Uuid::new_v4();
        let upload_url = format!("{}upload/out-{}", self.url, id);
        let download_url = format!("{}out-{}", self.url, id);
        Ok(DistSlot {
            upload_url,
            download_url,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    /// Deployment id in url like form.
    pub id: String,
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry_points: Vec<EntryPoint>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mount_points: Vec<MountPoint>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct EntryPoint {
    pub id: String,
    pub wasm_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum MountPoint {
    Ro(String),
    Rw(String),
    Wo(String),
}

impl MountPoint {
    pub fn path(&self) -> &str {
        match self {
            MountPoint::Ro(path) => path,
            MountPoint::Rw(path) => path,
            MountPoint::Wo(path) => path,
        }
    }
}

fn build_image(wasm_path: &Path) -> anyhow::Result<Vec<u8>> {
    let name_ws = wasm_path.file_name().unwrap().to_string_lossy();

    let m = Manifest {
        id: "wasm-runner/-/todo".to_string(),
        name: name_ws.to_string(),
        entry_points: vec![EntryPoint {
            id: "main".to_string(),
            wasm_path: name_ws.to_string(),
        }],
        mount_points: vec![MountPoint::Ro("in".into()), MountPoint::Rw("out".into())],
    };

    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zw.start_file(
        "manifest.json",
        zip::write::FileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    serde_json::to_writer_pretty(&mut zw, &m)?;
    zw.start_file(
        name_ws.as_ref(),
        zip::write::FileOptions::default().compression_method(CompressionMethod::Bzip2),
    )?;
    std::io::copy(
        &mut fs::OpenOptions::new().read(true).open(wasm_path)?,
        &mut zw,
    )?;
    let data = zw.finish()?.into_inner();
    Ok(data)
}

async fn push_image(
    hub_url: Arc<str>,
    image: Vec<u8>,
) -> Result<String, awc::error::WsClientError> {
    let c = awc::Client::new();

    let hex = format!("{:x}", <sha3::Sha3_224 as Digest>::digest(image.as_slice()));
    let download_url = format!("{}/app-{}.yimg", hub_url, &hex[0..8]);
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

fn build_demand(node_name: &str, wasm_url: &str) -> Demand {
    Demand {
        properties: serde_json::json!({
            "golem": {
                "node": {
                    "id": {
                        "name": node_name
                    },
                },
                "srv": {
                    "comp":{
                        "wasm": {
                            "task_package": wasm_url
                        }
                    }
                }
            }
        }),
        constraints: r#"(&
            (golem.inf.mem.gib>0.5)
            (golem.inf.storage.gib>1)
            (golem.com.pricing.model=linear)
        )"#
        .to_string(),

        demand_id: Default::default(),
        requestor_id: Default::default(),
    }
}

struct AgreementProducer {
    subscription_id: String,
    api: MarketRequestorApi,
    my_demand: Demand,
    pending: Vec<oneshot::Sender<String>>,
}

impl Actor for AgreementProducer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let _ = ctx.run_interval(Duration::from_secs(10), |act, ctx| {
            if !act.pending.is_empty() {
                let requestor_api = act.api.clone();
                let subscription_id = act.subscription_id.clone();
                let me = ctx.address();

                let _ = ctx.spawn(
                    async move {
                        let events = requestor_api
                            .collect(&subscription_id, Some(8.0), Some(5))
                            .await
                            .unwrap();
                        for event in events {
                            let _ = me.send(ProcessEvent(event)).await;
                        }
                    }
                    .into_actor(act),
                );
            }
        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let subscription_id = self.subscription_id.clone();
        let api = self.api.clone();
        ctx.wait(
            async move {
                if let Err(e) = api.unsubscribe(&subscription_id).await {
                    log::error!("unsubscribe error: {}", e);
                }
            }
            .into_actor(self),
        );
    }
}

struct ProcessEvent(RequestorEvent);

impl Message for ProcessEvent {
    type Result = ();
}

struct NewAgreement;

impl Message for NewAgreement {
    type Result = Result<String, anyhow::Error>;
}

impl Handler<NewAgreement> for AgreementProducer {
    type Result = ActorResponse<Self, String, anyhow::Error>;

    fn handle(&mut self, msg: NewAgreement, ctx: &mut Self::Context) -> Self::Result {
        let (tx, rx) = oneshot::channel();
        self.pending.push(tx);

        ActorResponse::r#async(
            async move {
                let agreement_id = rx.await?;
                Ok(agreement_id)
            }
            .into_actor(self),
        )
    }
}

impl Handler<ProcessEvent> for AgreementProducer {
    type Result = MessageResult<ProcessEvent>;

    fn handle(&mut self, msg: ProcessEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.0 {
            RequestorEvent::ProposalEvent {
                event_date: _,
                proposal,
            } => {
                log::debug!(
                    "processing ProposalEvent [{:?}] with state: {:?}",
                    proposal.proposal_id,
                    proposal.state
                );
                if proposal.state.unwrap_or(ProposalState::Initial) == ProposalState::Initial {
                    if proposal.prev_proposal_id.is_some() {
                        log::error!(
                            "Proposal in Initial state but with prev id: {:#?}",
                            proposal
                        );
                        return MessageResult(());
                    }
                    let bespoke_proposal = proposal.counter_demand(self.my_demand.clone()).unwrap();
                    let requestor_api = self.api.clone();
                    let subscription_id = self.subscription_id.clone();
                    let f = async move {
                        let new_proposal_id = requestor_api
                            .counter_proposal(&bespoke_proposal, &subscription_id)
                            .await
                            .unwrap();
                        log::debug!("new proposal id = {}", new_proposal_id);
                    };
                    let _ = ctx.spawn(f.into_actor(self));
                } else {
                    // Try to create agreement
                    if self.pending.is_empty() {
                        return MessageResult(());
                    }
                    let new_agreement_id = proposal.proposal_id().unwrap().clone();
                    let new_agreement = AgreementProposal::new(
                        new_agreement_id.clone(),
                        Utc::now() + chrono::Duration::hours(2),
                    );
                    let me = ctx.address();

                    let requestor_api = self.api.clone();
                    let me = ctx.address();
                    let _ = ctx.spawn(
                        async move {
                            let _ack = requestor_api
                                .create_agreement(&new_agreement)
                                .await
                                .unwrap();
                            log::info!("confirm agreement = {}", new_agreement_id);
                            requestor_api
                                .confirm_agreement(&new_agreement_id)
                                .await
                                .unwrap();
                            log::info!("wait for agreement = {}", new_agreement_id);
                            requestor_api
                                .wait_for_approval(&new_agreement_id, Some(7.879))
                                .await
                                .unwrap();
                            log::info!("agreement = {} CONFIRMED!", new_agreement_id);
                            new_agreement_id
                        }
                        .into_actor(self)
                        .then(|agreement_id, act, ctx| {
                            if let Some(mut s) = act.pending.pop() {
                                s.send(agreement_id);
                            }
                            fut::ready(())
                        }),
                    );
                }
            }
            _ => {
                log::warn!("invalid response");
            }
        }
        MessageResult(())
    }
}

async fn agreement_producer(
    market_api: &MarketRequestorApi,
    demand: &Demand,
) -> anyhow::Result<Addr<AgreementProducer>> {
    let subscription_id = market_api.subscribe(demand).await?;
    log::info!("sub_id={}", subscription_id);
    let producer = AgreementProducer {
        subscription_id,
        api: market_api.clone(),
        my_demand: demand.clone(),
        pending: Default::default(),
    };

    Ok(producer.start())
}

async fn allocate_funds_for_task(
    payment_api: &ya_client::payment::requestor::RequestorApi,
    n_tasks: usize,
) -> anyhow::Result<String> {
    let new_allocation = model::payment::NewAllocation {
        total_amount: ((n_tasks * 2) as u64).into(),
        timeout: None,
        make_deposit: false,
    };
    let allocation = payment_api.create_allocation(&new_allocation).await?;
    log::info!("Allocated {} GNT.", &allocation.total_amount);
    Ok(allocation.allocation_id)
}

#[derive(Debug)]
struct TaskResult {
    agreement_id: String,
    task_def: TaskDef,
}

async fn process_task(
    storage: DistStorage,
    client: WebClient,
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
        eprintln!("output={}", blob_path);
        let slot = storage.download_slot().await?;
        commands.push(serde_json::json!({"transfer": {
            "from": format!("container:/out/{}", blob_path),
            "to": slot.url()
        }}));
        outputs.push((slot, merge_path.join(blob_path)))
    }
    let output_slot = storage.download_slot().await?;
    commands.push(serde_json::json!({"transfer": {
        "from": format!("container:/out/task.json"),
        "to": output_slot.url()
    }}));

    let commands_cnt = commands.len();
    let script_text = serde_json::to_string_pretty(&commands)?;
    eprintln!("script=[{}]", script_text);
    let script = ya_client::model::activity::ExeScriptRequest::new(script_text);

    let activity_api = client.interface::<ya_client::activity::ActivityRequestorApi>()?;

    let agreement_id = a.send(NewAgreement).await??;
    let activity_id = match activity_api.control().create_activity(&agreement_id).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("activity create error: {}", e);
            return Err(e.into());
        }
    };

    let batch_id = activity_api
        .control()
        .exec(script.clone(), &activity_id)
        .await?;
    loop {
        let state = activity_api.state().get_state(&activity_id).await?;
        if !state.alive() {
            log::info!("activity {} is NOT ALIVE any more.", activity_id);
            break;
        }

        log::info!("activity {} state: {:?}", activity_id, state);
        let results = activity_api
            .control()
            .get_exec_batch_results(&activity_id, &batch_id, Some(7))
            .await?;

        log::info!("batch results {:?}", results);

        if results.len() >= commands_cnt {
            break;
        }

        tokio::time::delay_for(Duration::from_millis(700)).await;
    }

    // TODO: task output path resolve
    let task_def = output_slot.download_json().await?;

    for (slot, output) in outputs {
        eprintln!("downloading: {}", output.display());
        slot.download(&output).await?;
    }
    if let Err(e) = activity_api.control().destroy_activity(&activity_id).await {
        log::error!("fail to destroy activity: {}", e);
    }

    Ok(TaskResult {
        agreement_id,
        task_def,
    })
}

pub fn run(
    hub_addr: Option<String>,
    token: Option<String>,
    engine: impl Engine,
    wasm_path: &Path,
    args: &[String],
) -> anyhow::Result<()> {
    let _ = dotenv::dotenv().ok();
    let token = match token {
        Some(token) => token,
        None => std::env::var("YAGNA_APPKEY")?,
    };
    let client = ya_client::web::WebClient::with_token(&token)?;

    let mut sys = System::new("wasm -runner");
    let mut w = WorkDir::new("lwg")?;
    let image = build_image(&wasm_path)?;
    //let hub_url: Arc<str> = format!("http://{}", hub_addr).into();
    let output_path = w.split_output()?;
    {
        let mut split_args = Vec::new();
        split_args.push("split".to_owned());
        split_args.push("/task_dir/".to_owned());
        split_args.extend(args.iter().cloned());
        run_local_code(engine.clone(), wasm_path, &output_path, split_args)?;
    }

    let tasks_path = output_path.join("tasks.json");

    eprintln!("reading: {}", tasks_path.display());
    let tasks: Vec<gwasm_dispatcher::TaskDef> =
        serde_json::from_reader(fs::OpenOptions::new().read(true).open(tasks_path)?)?;

    let merge_path = w.merge_path()?;
    let output_file = merge_path.join("tasks.json");
    let merge_path_ref = merge_path.clone();

    let storage_server: Arc<str> = "http://34.244.4.185:8000/".into();
    let payment_api: ya_client::payment::requestor::RequestorApi = client.interface()?;
    let task_output_path = output_path.clone();
    let r = sys.block_on(async move {
        // TODO: Catch error
        let image = push_image(storage_server.clone(), image).await.unwrap();
        eprintln!("image={}", image);

        let node_name = "test1";
        let my_demand = build_demand(node_name, &image);
        let market_api: ya_client::market::MarketRequestorApi = client.interface()?;
        let a = agreement_producer(&market_api, &my_demand).await?;
        let storage = DistStorage::new(storage_server);
        let output_tasks = merge_path_ref.join("tasks.json");
        let agreements = futures::future::join_all(tasks.into_iter().map(|t| {
            process_task(
                storage.clone(),
                client.clone(),
                a.clone(),
                task_output_path.clone(),
                merge_path_ref.clone(),
                t,
            )
        }))
        .await;

        let mut tasks = Vec::new();
        for agr in agreements {
            let result = agr?;
            eprintln!("result={:?}", result);
            tasks.push(result.task_def);
        }

        std::fs::write(output_tasks, serde_json::to_vec_pretty(&tasks)?)?;

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
            engine.clone(),
            wasm_path,
            merge_path.parent().unwrap(),
            merge_args,
        )?;
    }

    Ok(())
}

use actix::prelude::*;
use chrono::Utc;
use futures::channel::oneshot;
use futures::prelude::*;
use futures::SinkExt;
use std::time::Duration;
use ya_client::market::MarketRequestorApi;
use ya_client::model::market::{
    proposal::State as ProposalState, AgreementProposal, Demand, Proposal, RequestorEvent,
};

pub struct AgreementProducer {
    subscription_id: String,
    api: MarketRequestorApi,
    my_demand: Demand,
    pending: Vec<oneshot::Sender<String>>,
}

impl Actor for AgreementProducer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.process_events(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        log::info!("Stopping");
        let subscription_id = self.subscription_id.clone();
        let api = self.api.clone();
        let _ = Arbiter::spawn(
            async move {
                if let Err(e) = api.unsubscribe(&subscription_id).await {
                    log::error!("unsubscribe error: {}", e);
                }
                log::info!("unsubscribe done");
            });

    }
}

impl AgreementProducer {
    fn process_events(&mut self, ctx: &mut Context<Self>) {
        let me = ctx.address();
        let subscription_id = self.subscription_id.clone();
        let requestor_api = self.api.clone();
        if !self.pending.is_empty() {
            let _ = ctx.spawn(
                async move {
                    let run_after = tokio::time::Instant::now() + Duration::from_secs(8);
                    let events = match requestor_api
                        .collect(&subscription_id, Some(8.0), Some(5))
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("fail to get market events: {}", e);
                            return;
                        }
                    };
                    if events.is_empty() {
                        tokio::time::delay_until(run_after).await
                    }
                    for event in events {
                        let _ = me.send(ProcessEvent(event)).await;
                    }
                }
                .into_actor(self)
                .then(|r, act, ctx| {
                    act.process_events(ctx);
                    fut::ready(())
                }),
            );
        } else {
            let _ = ctx.run_later(Duration::from_secs(1), |act, ctx| act.process_events(ctx));
        }
    }
}

struct ProcessEvent(RequestorEvent);

impl Message for ProcessEvent {
    type Result = ();
}

pub struct NewAgreement;

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
                    let bespoke_proposal = match proposal.counter_demand(self.my_demand.clone()) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "problem with proposal: {:?} from {:?}: {}",
                                proposal.proposal_id,
                                proposal.issuer_id,
                                e
                            );
                            return MessageResult(());
                        }
                    };
                    let provider_id = proposal.issuer_id.clone().unwrap_or_default();
                    let requestor_api = self.api.clone();
                    let subscription_id = self.subscription_id.clone();
                    let f = async move {
                        let new_proposal_id = match requestor_api
                            .counter_proposal(&bespoke_proposal, &subscription_id)
                            .await
                        {
                            Ok(v) => v,
                            Err(e) => {
                                log::error!("counter_proposal fail: {}", e);
                                return;
                            }
                        };
                        log::debug!("new proposal id = {} for: {}", new_proposal_id, provider_id);
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
                    let mut slot = match self.pending.pop() {
                        Some(slot) => slot,
                        None => return MessageResult(()),
                    };
                    let _ = ctx.spawn(
                        async move {
                            if let Err(e) = async {
                                let _ack = requestor_api.create_agreement(&new_agreement).await?;
                                log::info!("confirm agreement = {}", new_agreement_id);
                                requestor_api.confirm_agreement(&new_agreement_id).await?;
                                log::info!("wait for agreement = {}", new_agreement_id);
                                requestor_api
                                    .wait_for_approval(&new_agreement_id, Some(7.879))
                                    .await?;
                                Ok::<_, anyhow::Error>(())
                            }
                            .await
                            {
                                log::error!("fail to negotiate agreement: {}", new_agreement_id);
                                Err(slot)
                            } else {
                                log::info!("agreement = {} CONFIRMED!", new_agreement_id);
                                let _ = slot.send(new_agreement_id);
                                Ok(())
                            }
                        }
                        .into_actor(self)
                        .then(|r, act, _ctx| {
                            if let Err(slot) = r {
                                act.pending.push(slot);
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

pub struct Kill;

impl Message for Kill {
    type Result = ();
}

impl Handler<Kill> for AgreementProducer {
    type Result = MessageResult<Kill>;

    fn handle(&mut self, _: Kill, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
        MessageResult(())
    }
}

pub async fn agreement_producer(
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
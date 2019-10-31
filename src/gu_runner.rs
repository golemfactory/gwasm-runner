use crate::local_runner::run_local_code;
use crate::workdir::WorkDir;
use actix::prelude::*;
use failure::{bail, Fallible};
use futures::unsync::oneshot;
use futures::Async;
use gu_client::model::envman::{Command, CreateSession, ResourceFormat};
use gu_client::{r#async as guc, NodeId};
use gu_wasm_env_api::{EntryPoint, Manifest, MountPoint, RuntimeType};
use gwasm_api::TaskDef;
use serde::Serialize;
use sp_wasm_engine::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Cursor, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::CompressionMethod;

// 1. Image cache [TODO]
// 2.
//

fn build_image(wasm_path: &Path, js_path: &Path) -> Fallible<Vec<u8>> {
    let name_ws = wasm_path.file_name().unwrap().to_string_lossy();
    let name_js = js_path.file_name().unwrap().to_string_lossy();

    let m = Manifest {
        id: "unlimited.golem.network/wasm-runner/-/todo".to_string(),
        name: name_ws.to_string(),
        main: None,
        entry_points: vec![EntryPoint {
            id: "job".to_string(),
            wasm_path: name_ws.to_string(),
            args_prefix: vec![],
        }],
        runtime: RuntimeType::Emscripten,
        mount_points: vec![MountPoint::Ro("/in".into()), MountPoint::Rw("/out".into())],
        work_dir: None,
    };

    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zw.start_file(
        "gu-package.json",
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
    zw.start_file(
        name_js.as_ref(),
        zip::write::FileOptions::default().compression_method(CompressionMethod::Bzip2),
    )?;
    std::io::copy(
        &mut fs::OpenOptions::new().read(true).open(js_path)?,
        &mut zw,
    )?;
    let data = zw.finish()?.into_inner();
    Ok(data)
}

fn push_image(
    hub_url: Arc<str>,
    image: Vec<u8>,
) -> impl Future<Item = (String, String), Error = failure::Error> {
    let c = awc::Client::new();

    c.post(format!("{}/repo", hub_url))
        .content_length(image.len() as u64)
        .content_type("application/octet-stream")
        .send_body(image)
        .map_err(|e| failure::err_msg(e.to_string()))
        .and_then(move |mut r| {
            r.json()
                .map_err(|e| failure::err_msg(e.to_string()))
                .and_then(move |image_id: String| {
                    let hash = format!("sha1:{}", image_id);
                    Ok((format!("{}/repo/{}", hub_url, image_id), hash))
                })
        })
}

struct Work {
    commands: Vec<Command>,
    meta_blob: guc::Blob,
    outputs: Vec<(guc::Blob, PathBuf)>,
    task_path: PathBuf,
    merge_path: PathBuf,
}

fn download_blob(
    blob: &guc::Blob,
    destination: &Path,
) -> impl Future<Item = (), Error = failure::Error> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .unwrap();
    blob.download().from_err().for_each(move |chunk| {
        f.write_all(chunk.as_ref())?;
        Ok(())
    })
}

impl Work {
    fn download_results(&self) -> impl Future<Item = TaskDef, Error = failure::Error> {
        let files = if self.outputs.is_empty() {
            futures::future::Either::A(futures::future::ok(()))
        } else {
            futures::future::Either::B(
                futures::future::join_all(
                    self.outputs
                        .iter()
                        .map(|(blob, output)| download_blob(blob, output))
                        .collect::<Vec<_>>(),
                )
                .and_then(|_| Ok(())),
            )
        };

        let task_def = self
            .meta_blob
            .download()
            .map_err(failure::err_msg)
            .fold(Vec::new(), |mut v, b| {
                v.extend_from_slice(b.as_ref());
                futures::future::ok::<_, failure::Error>(v)
            })
            .and_then(|d| -> Result<TaskDef, _> {
                serde_json::from_slice(d.as_ref()).map_err(failure::err_msg)
            });

        let task_path = self.task_path.clone();
        let merge_path = self.merge_path.clone();

        Future::join(files, task_def).and_then(move |(_, task_def)| {
            task_def
                .rebase_to(&task_path, &merge_path)
                .map_err(failure::err_msg)
        })
    }
}

enum WorkPeerState {
    Added,
    Pending(Arc<Work>, ReplyRef),
    Work(guc::PeerSession),
    Backoff,
}

impl WorkPeerState {
    fn is_free(&self) -> bool {
        if let WorkPeerState::Added = self {
            true
        } else {
            false
        }
    }
}

type ReplyRef = oneshot::Sender<Result<TaskDef, failure::Error>>;

struct WorkManager {
    session: guc::HubSessionRef,
    deployment_desc: CreateSession,
    peers: HashMap<NodeId, WorkPeerState>,
    todo: VecDeque<(Arc<Work>, ReplyRef)>,
}

impl Actor for WorkManager {
    type Context = Context<Self>;

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let _ = self.session.clone().into_inner();
        for (node_id, s) in &self.peers {
            match s {
                WorkPeerState::Added => eprintln!("{:?} - added", node_id),
                WorkPeerState::Pending(_, _) => eprintln!("{:?} - pending", node_id),
                WorkPeerState::Work(_) => eprintln!("{:?} - working", node_id),
                WorkPeerState::Backoff => eprintln!("{:?} - error", node_id),
            }
        }
        eprintln!("work done: {} task pending", self.todo.len());
        ctx.wait(
            self.session
                .deref()
                .clone()
                .delete()
                .then(|_| Ok(()))
                .into_actor(self),
        );
    }
}

impl WorkManager {
    fn schedule_tasks(&mut self, ctx: &mut <Self as Actor>::Context) {
        let mut free_peers = self
            .peers
            .iter()
            .filter_map(|(node_id, state)| {
                if state.is_free() {
                    Some(*node_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if free_peers.is_empty() {
            log::info!("no free nodes");
            return;
        }

        while let Some((work, reply)) = self.todo.pop_front() {
            self.schedule_to(free_peers.pop().unwrap(), work, reply, ctx);
            if free_peers.is_empty() {
                break;
            }
        }
    }

    fn schedule_to(
        &mut self,
        node_id: NodeId,
        work: Arc<Work>,
        reply: ReplyRef,
        ctx: &mut <Self as Actor>::Context,
    ) {
        log::info!("schedule work to {:?}", node_id);
        if let Some(state) = self.peers.get_mut(&node_id) {
            *state = WorkPeerState::Pending(work, reply);
            let _ = ctx.spawn(
                self.session
                    .peer(node_id)
                    .new_session(self.deployment_desc.clone())
                    .into_actor(self)
                    .then(move |r, act, ctx| match r {
                        Ok(deployment) => fut::ok(act.deployment_ready(node_id, deployment, ctx)),
                        Err(_) => {
                            log::error!("failed to deploy on node: {:?}", node_id);
                            fut::ok(act.backoff_node(node_id))
                        }
                    }),
            );
        } else {
            log::error!("invalid node state: {:?}", node_id);
        }
    }

    fn deployment_ready(
        &mut self,
        node_id: NodeId,
        deployment: guc::PeerSession,
        ctx: &mut <Self as Actor>::Context,
    ) {
        log::info!("running work on {:?}", node_id);
        if let Some(s) = self
            .peers
            .insert(node_id, WorkPeerState::Work(deployment.clone()))
        {
            let (work, reply) = match s {
                WorkPeerState::Pending(work, reply) => (work, reply),
                _ => {
                    log::error!("invalid peer state, dropping deloyment");
                    ctx.spawn(deployment.delete().map_err(|_| ()).into_actor(self));
                    return;
                }
            };
            let _ = ctx.spawn(
                deployment
                    .update(work.commands.clone())
                    .from_err()
                    .into_actor(self)
                    .then(
                        move |r: Result<Vec<String>, gu_client::error::Error>, act, ctx| {
                            log::debug!("deployment resolved: {:?}", r);
                            ctx.spawn(deployment.delete().then(|_| Ok(())).into_actor(act));
                            act.release_node(node_id, ctx);
                            if let Err(e) = r {
                                let _ = reply.send(Err(e.into()));
                                return fut::Either::B(fut::ok(()));
                            }

                            fut::Either::A(
                                work.download_results()
                                    .and_then(move |task_def| {
                                        let _ = reply.send(Ok(task_def));
                                        Ok(())
                                    })
                                    .into_actor(act),
                            )
                        },
                    )
                    .map_err(|e, _, _| log::error!("download results error: {}", e)),
            );
        }
    }

    fn backoff_node(&mut self, node_id: NodeId) {
        log::info!("backoff node: {:?}", node_id);
        if let Some(s) = self.peers.insert(node_id, WorkPeerState::Backoff) {
            if let WorkPeerState::Pending(work, reply) = s {
                self.todo.push_back((work, reply))
            }
        }
    }

    fn release_node(&mut self, node_id: NodeId, ctx: &mut <Self as Actor>::Context) {
        log::info!("release node: {:?}", node_id);
        if let Some(s) = self.peers.get_mut(&node_id) {
            *s = WorkPeerState::Added;
        }
        self.schedule_tasks(ctx)
    }
}

struct RunWork(Work);

impl Message for RunWork {
    type Result = Result<TaskDef, failure::Error>;
}

impl Handler<RunWork> for WorkManager {
    type Result = ActorResponse<Self, TaskDef, failure::Error>;

    fn handle(&mut self, msg: RunWork, ctx: &mut Self::Context) -> Self::Result {
        let (tx, rx) = futures::unsync::oneshot::channel();

        self.todo.push_back((Arc::new(msg.0), tx));
        self.schedule_tasks(ctx);

        ActorResponse::r#async(rx.flatten().into_actor(self))
    }
}

struct StopManager;

impl Message for StopManager {
    type Result = Result<(), failure::Error>;
}

impl Handler<StopManager> for WorkManager {
    type Result = ActorResponse<Self, (), failure::Error>;

    fn handle(&mut self, _: StopManager, _ctx: &mut Self::Context) -> Self::Result {
        let session = self.session.deref().clone();
        ActorResponse::r#async(session.delete().from_err().into_actor(self).and_then(
            |_, _, ctx| {
                ctx.stop();
                fut::ok(())
            },
        ))
    }
}

impl WorkManager {
    fn new(
        session: guc::HubSessionRef,
        peers: Vec<NodeId>,
        deployment_desc: CreateSession,
    ) -> Addr<WorkManager> {
        let peers = peers
            .into_iter()
            .map(|node_id| (node_id, WorkPeerState::Added))
            .collect();
        WorkManager {
            session,
            peers,
            deployment_desc,
            todo: Default::default(),
        }
        .start()
    }
}

fn file_stream(f: &Path) -> impl Stream<Item = bytes::Bytes, Error = io::Error> {
    let mut inf = fs::OpenOptions::new().read(true).open(f).unwrap();
    let mut buf = bytes::BytesMut::with_capacity(40960);

    futures::stream::poll_fn(move || {
        buf.reserve(4096);
        buf.resize(buf.capacity(), 0);
        //eprintln!("reading file: ");
        let bytes = inf.read(&mut buf[..])?;
        //eprintln!("got {} bytes", bytes);
        if bytes == 0 {
            return Ok(Async::Ready(None));
        }
        let result = buf.split_to(bytes).freeze();
        //eprintln!("sending: {}", result.len());
        Ok(Async::Ready(Some(result)))
    })
}

fn json_stream<T: Serialize>(object: &T) -> impl Stream<Item = bytes::Bytes, Error = io::Error> {
    let bytes = serde_json::to_vec(object)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        .map(|v| v.into());

    futures::stream::once(bytes)
}

pub fn run(hub_addr: String, wasm_path: &Path, args: &[String]) -> Fallible<()> {
    {
        let mut sys = System::new("GU-wasm -runner");
        let engine_ref = Sandbox::init_ejs()?;
        let mut w = WorkDir::new("gu")?;

        let js_path = wasm_path.with_extension("js");

        if !js_path.exists() {
            bail!("file not found: {}", js_path.display())
        }

        let image = build_image(&wasm_path, &js_path)?;

        let hub_url: Arc<str> = format!("http://{}", hub_addr).into();

        let output_path = w.split_output()?;
        {
            let mut split_args = Vec::new();
            split_args.push("split".to_owned());
            split_args.push("/task_dir/".to_owned());
            split_args.extend(args.iter().cloned());
            run_local_code(
                engine_ref.clone(),
                wasm_path,
                &js_path,
                &output_path,
                split_args,
            )?;
        }

        let tasks_path = output_path.join("tasks.json");

        eprintln!("reading: {}", tasks_path.display());
        let tasks: Vec<gwasm_api::TaskDef> =
            serde_json::from_reader(fs::OpenOptions::new().read(true).open(tasks_path)?)?;

        let merge_path = w.merge_path()?;
        let output_file = merge_path.join("tasks.json");
        let merge_path_ref = merge_path.clone();

        let image_fut = push_image(hub_url.clone(), image)
            .map_err(failure::err_msg)
            .and_then(|(image_url, image_hash)| {
                eprintln!("got image: {}", image_url);
                let c = gu_client::r#async::HubConnection::from_addr(hub_addr).unwrap();
                let session = c
                    .new_session(gu_client::model::session::HubSessionSpec {
                        expires: None,
                        allocation: Default::default(),
                        name: Some(format!("work for {}", wasm_path.display())),
                        tags: vec!["gu:wasm".to_string(), "gu:wasm:runner".to_string()]
                            .into_iter()
                            .collect(),
                    })
                    .map_err(failure::err_msg);
                let peers = c.list_peers().map_err(failure::err_msg);

                session.join4(
                    peers,
                    futures::future::ok(image_url),
                    futures::future::ok(image_hash),
                )
            });

        let work = image_fut
            .and_then(|(session, peers, image_url, image_hash)| {
                session
                    .add_peers(peers.map(|n| n.node_id))
                    .from_err()
                    .and_then(move |nodes| Ok((session, nodes, image_url, image_hash)))
            })
            .from_err()
            .and_then(
                move |(session, nodes, image_url, image_hash): (guc::HubSessionRef, _, _, _)| {
                    let session_for_update = session.clone();
                    let merge_path = merge_path_ref.clone();

                    futures::future::join_all(
                        tasks
                            .into_iter()
                            .map(move |task| {
                                let task_dir = w.new_task().unwrap();

                                let task_desc = task.clone();
                                let input_data_iter = task
                                    .blobs()
                                    .into_iter()
                                    .map(|blob_id| {
                                        let blob_path = output_path.join(blob_id);
                                        let s = file_stream(&blob_path);
                                        let file_path = format!("/in/{}", blob_id);
                                        session.new_blob().and_then(move |b| {
                                            eprintln!("new blob: {}", b.id());
                                            b.upload_from_stream(s).and_then(move |()| {
                                                Ok(Command::DownloadFile {
                                                    uri: b.uri(),
                                                    file_path,
                                                    format: ResourceFormat::Raw,
                                                })
                                            })
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                let input_data = futures::future::join_all(input_data_iter);

                                let input_meta = session.new_blob().and_then(move |b| {
                                    b.upload_from_stream(json_stream(&task_desc)).and_then(
                                        move |()| {
                                            Ok(Command::DownloadFile {
                                                uri: b.uri(),
                                                file_path: "/in/task.json".to_string(),
                                                format: ResourceFormat::Raw,
                                            })
                                        },
                                    )
                                });
                                let output_data = futures::future::join_all(
                                    task.outputs()
                                        .into_iter()
                                        .map(|blob_id| {
                                            let file_path = format!("/out/{}", blob_id);
                                            let output_path = task_dir.join(blob_id);
                                            session.new_blob().and_then(move |b| {
                                                log::debug!("new output {} {}", file_path, b.id());
                                                Ok((
                                                    Command::UploadFile {
                                                        uri: b.uri(),
                                                        file_path,
                                                        format: ResourceFormat::Raw,
                                                    },
                                                    b,
                                                    output_path,
                                                ))
                                            })
                                        })
                                        .collect::<Vec<_>>(),
                                );

                                let task_dir_ref = task_dir.clone();
                                let merge_path_ref = merge_path.clone();

                                let output_meta = session.new_blob().and_then(move |b| {
                                    Ok((
                                        Command::UploadFile {
                                            uri: b.uri(),
                                            file_path: "/out/task.json".to_string(),
                                            format: ResourceFormat::Raw,
                                        },
                                        b,
                                        task_dir_ref,
                                        merge_path_ref,
                                    ))
                                });

                                input_meta
                                    .join4(input_data, output_meta, output_data)
                                    .and_then(
                                        |(
                                            in_meta,
                                            in_data,
                                            (out_meta, out_meta_blob, task_path, merge_path),
                                            out_data,
                                        )| {
                                            let mut commands = Vec::new();
                                            let mut downloads = Vec::new();
                                            commands.push(in_meta);
                                            commands.extend(in_data);
                                            commands.push(Command::Exec {
                                                executable: "job".to_string(),
                                                args: vec![
                                                    "exec".to_string(),
                                                    "/in/task.json".to_string(),
                                                    "/out/task.json".to_string(),
                                                ],
                                            });
                                            commands.push(out_meta);
                                            for (command, blob, out_path) in out_data {
                                                commands.push(command);
                                                downloads.push((blob, out_path));
                                            }
                                            Ok(Work {
                                                commands,
                                                meta_blob: out_meta_blob,
                                                outputs: downloads,
                                                task_path,
                                                merge_path,
                                            })
                                        },
                                    )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .from_err()
                    .and_then(move |r| {
                        log::info!("running code: {} tasks", r.len());
                        let session = session_for_update.clone();
                        let image = gu_client::model::envman::Image {
                            url: image_url,
                            hash: image_hash,
                        };
                        let deployment_desc = CreateSession {
                            env_type: "wasm".to_string(),
                            image: image.clone(),
                            name: "".to_string(),
                            tags: vec![],
                            note: None,
                            options: (),
                        };

                        let w = WorkManager::new(session.clone(), nodes, deployment_desc);
                        let we = w.clone();
                        futures::future::join_all(
                            r.into_iter()
                                .map(move |work| w.send(RunWork(work)).flatten()),
                        )
                        .and_then(move |tasks| Ok((tasks, we)))
                    })
                },
            )
            .and_then(|(tasks, w)| w.send(StopManager).then(|_| Ok(tasks)))
            .map_err(|e| {
                log::error!("fail: {:?}", e);
                e
            });

        guc::disable_release();
        let tasks = sys.block_on(work)?;

        serde_json::to_writer_pretty(
            BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create_new(true)
                    .open(&output_file)?,
            ),
            &tasks,
        )?;
        {
            let mut merge_args = Vec::new();
            merge_args.push("merge".to_owned());
            merge_args.push("/task_dir/split/tasks.json".to_owned());
            merge_args.push("/task_dir/merge/tasks.json".to_owned());
            merge_args.push("--".to_owned());
            merge_args.extend(args.iter().cloned());
            run_local_code(
                engine_ref,
                wasm_path,
                &js_path,
                merge_path.parent().unwrap(),
                merge_args,
            )?;
        }
    }

    Ok(())
}

use hidapi::HidApi;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        mpsc::{self, Receiver, Sender},
        OnceLock,
    },
    thread,
};

type Task<T> = Box<dyn FnOnce(&mut T) + Send + 'static>;

struct OwnedWorker<T: 'static> {
    sender: Sender<Task<T>>,
}

impl<T: 'static> OwnedWorker<T> {
    fn start(
        thread_name: &str,
        initialize: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<Self, String> {
        let (task_sender, task_receiver) = mpsc::channel::<Task<T>>();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);

        thread::Builder::new()
            .name(thread_name.to_owned())
            .spawn(move || match initialize() {
                Ok(mut state) => {
                    let _ = ready_sender.send(Ok(()));
                    while let Ok(task) = task_receiver.recv() {
                        let _ = catch_unwind(AssertUnwindSafe(|| task(&mut state)));
                    }
                }
                Err(error) => {
                    let _ = ready_sender.send(Err(error));
                }
            })
            .map_err(|error| format!("Could not start {thread_name}: {error}"))?;

        ready_receiver
            .recv()
            .map_err(|_| format!("{thread_name} stopped during startup"))??;

        Ok(Self {
            sender: task_sender,
        })
    }

    fn dispatch<R: Send + 'static>(
        &self,
        task: impl FnOnce(&mut T) -> R + Send + 'static,
    ) -> Result<Receiver<R>, String> {
        let (result_sender, result_receiver) = mpsc::sync_channel(1);
        self.sender
            .send(Box::new(move |state| {
                let _ = result_sender.send(task(state));
            }))
            .map_err(|_| "The HID service is unavailable. Relaunch kbd.ctrl.".to_owned())?;
        Ok(result_receiver)
    }
}

static HID_WORKER: OnceLock<Result<OwnedWorker<HidApi>, String>> = OnceLock::new();

fn worker() -> Result<&'static OwnedWorker<HidApi>, String> {
    HID_WORKER
        .get_or_init(|| {
            OwnedWorker::start("kbd-hid-service", || {
                HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub async fn run<R: Send + 'static>(
    operation: &'static str,
    task: impl FnOnce(&HidApi) -> Result<R, String> + Send + 'static,
) -> Result<R, String> {
    let receiver = worker()?.dispatch(move |api| {
        api.refresh_devices()
            .map_err(|error| format!("HID device refresh failed: {error}"))?;
        task(api)
    })?;

    tauri::async_runtime::spawn_blocking(move || {
        receiver
            .recv()
            .map_err(|_| format!("The HID service stopped while running {operation}."))
    })
    .await
    .map_err(|error| format!("The {operation} task could not finish: {error}"))??
}

#[cfg(test)]
mod tests {
    use super::OwnedWorker;
    use std::{sync::mpsc, thread};

    #[test]
    fn worker_initialization_and_tasks_stay_on_one_thread() {
        let worker = OwnedWorker::start("test-owned-worker", || Ok(thread::current().id()))
            .expect("worker should start");

        for _ in 0..3 {
            let receiver = worker
                .dispatch(|owner| *owner == thread::current().id())
                .expect("task should be accepted");
            assert!(receiver.recv().expect("task should finish"));
        }
    }

    #[test]
    fn worker_serializes_tasks() {
        let worker = OwnedWorker::start("test-serial-worker", || Ok(Vec::<u8>::new()))
            .expect("worker should start");
        let (release_sender, release_receiver) = mpsc::sync_channel(0);

        let first = worker
            .dispatch(move |values| {
                values.push(1);
                release_receiver.recv().expect("release should arrive");
                values.clone()
            })
            .expect("first task should be accepted");
        let second = worker
            .dispatch(|values| {
                values.push(2);
                values.clone()
            })
            .expect("second task should be accepted");

        assert!(second.try_recv().is_err());
        release_sender.send(()).expect("first task should resume");
        assert_eq!(first.recv().expect("first task should finish"), vec![1]);
        assert_eq!(
            second.recv().expect("second task should finish"),
            vec![1, 2]
        );
    }
}

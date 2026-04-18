use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let child_token = token.child_token();

    let handle = tokio::spawn(worker(1, child_token));

    // simulate runtime
    sleep(Duration::from_millis(850)).await;

    // trigger shutdown
    token.cancel();

    // wait for task to finish cleanly
    let _ = handle.await;
}

async fn worker(task_id: u32, token: CancellationToken) {
    let mut iter = 1;
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("Task {task_id}: shutting down gracefully");
                break;
            }

            _ = async {
                // simulate multiple await points inside one iteration
                step_one().await;
                println!("Task 1-1 completed iteration");
                step_two().await;
                println!("Task 1-2 completed iteration");
                step_three().await;
                println!("Task 1-3 completed iteration");
            } => {
                println!("Task {iter}: completed iteration");
            }
        }
        iter = iter + 1;
    }

    println!("Task {task_id}: exited cleanly");
}

async fn step_one() {
    sleep(Duration::from_millis(200)).await;
}

async fn step_two() {
    sleep(Duration::from_millis(200)).await;
}

async fn step_three() {
    sleep(Duration::from_millis(200)).await;
}

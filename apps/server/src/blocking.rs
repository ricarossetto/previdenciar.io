use std::fmt;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug)]
pub enum BlockingGateError {
    Closed,
    Join,
}

impl fmt::Display for BlockingGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("o conjunto de tarefas bloqueantes foi encerrado"),
            Self::Join => f.write_str("a tarefa bloqueante terminou de forma inesperada"),
        }
    }
}

impl std::error::Error for BlockingGateError {}

pub struct BlockingGate {
    semaphore: Arc<Semaphore>,
}

impl BlockingGate {
    pub fn new() -> Self {
        let permits = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(4)
            .clamp(2, 4);
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
        }
    }

    pub fn with_permits(permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits.max(1))),
        }
    }

    pub fn permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub async fn run<T, F>(&self, work: F) -> Result<T, BlockingGateError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| BlockingGateError::Closed)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work()
        })
        .await
        .map_err(|_| BlockingGateError::Join)
    }
}

impl Default for BlockingGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concorrencia_bloqueante_nao_passa_do_semaforo() {
        let gate = Arc::new(BlockingGate::with_permits(2));
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..16 {
            let gate = gate.clone();
            let running = running.clone();
            let peak = peak.clone();
            handles.push(tokio::spawn(async move {
                gate.run(move || {
                    let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(20));
                    running.fetch_sub(1, Ordering::SeqCst);
                })
                .await
                .expect("tarefa bloqueante concluida");
            }));
        }
        for handle in handles {
            handle.await.expect("tarefa concluida");
        }
        let peak = peak.load(Ordering::SeqCst);
        assert!(peak <= 2, "pico de concorrencia {peak} passou do semaforo");
        assert!(peak >= 1);
        assert_eq!(gate.permits(), 2);
    }
}

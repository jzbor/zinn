use std::{collections::{HashMap, VecDeque}, sync::{Arc, Condvar, Mutex}};

use crate::JobRealization;

#[derive(Clone)]
pub struct Queue {
    inner: Arc<Mutex<InnerQueue>>,
    cond_fetch_job: Arc<Condvar>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum JobState {
    Ready,
    Failed,
    Running,
    Finished,
}

struct InnerQueue {
    jobs: VecDeque<JobRealization>,
    states: HashMap<JobRealization, JobState>,
    done: bool,
}

impl Queue {
    pub fn new() -> Self {
        let inner = InnerQueue {
            jobs: VecDeque::new(),
            states: HashMap::new(),
            done: false,
        };
        Queue {
            inner: Arc::new(Mutex::new(inner)),
            cond_fetch_job: Arc::new(Condvar::new()),
        }
    }

    pub fn enqueue(&self, job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        inner.jobs.push_back(job.clone());
        inner.states.insert(job, JobState::Ready);
        drop(inner);

        self.cond_fetch_job.notify_one();
    }

    pub fn fetch(&self) -> Option<JobRealization> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if inner.done && inner.all_tasks_distributed() {
                return None;
            }

            match inner.get_ready() {
                Some(job) => return Some(job),
                None => { inner = self.cond_fetch_job.wait(inner).unwrap(); },
            }
        }
    }

    pub fn finished(&self, finished_job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        inner.states.insert(finished_job, JobState::Finished);
    }

    pub fn failed(&self, failed_job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        inner.states.insert(failed_job, JobState::Failed);
    }

    pub fn done(&self) {
        self.inner.lock().unwrap().done = true;
        self.cond_fetch_job.notify_all();
    }
}

impl InnerQueue {
    fn get_ready(&mut self) -> Option<JobRealization> {
        let mut ret = None;
        for job in &self.jobs {
            if *self.states.get(job).unwrap() == JobState::Ready {
                ret = Some(job.clone())
            }
        }

        if let Some(job) = &ret {
            self.states.insert(job.clone(), JobState::Running);
        }

        ret
    }

    fn all_tasks_distributed(&self) -> bool {
        for (_, state) in &self.states {
            if *state != JobState::Finished
                    && *state != JobState::Running
                    && *state != JobState::Failed {
                return false;
            }
        }

        true
    }
}

use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex}};

use crate::JobRealization;

#[derive(Clone)]
pub struct Queue {
    inner: Arc<Mutex<InnerQueue>>,
    cond_fetch_job: Arc<Condvar>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum JobState {
    Ready,
    NotReady,
    Failed,
    Running,
    Finished,
}

struct InnerQueue {
    jobs: VecDeque<(JobRealization, JobState)>,
    done: bool,
}

impl Queue {
    pub fn new() -> Self {
        let inner = InnerQueue {
            jobs: VecDeque::new(),
            done: false,
        };
        Queue {
            inner: Arc::new(Mutex::new(inner)),
            cond_fetch_job: Arc::new(Condvar::new()),
        }
    }

    pub fn enqueue(&self, job: JobRealization) {
        self.inner.lock().unwrap()
            .jobs.push_back((job, JobState::Ready));
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
        for (job, state) in &mut inner.jobs {
            if finished_job == *job {
                *state = JobState::Finished;
            }
        }
    }

    pub fn failed(&self, failed_job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        for (job, state) in &mut inner.jobs {
            if failed_job == *job {
                *state = JobState::Finished;
            }
        }
    }

    pub fn done(&self) {
        self.inner.lock().unwrap().done = true;
        self.cond_fetch_job.notify_all();
    }
}

impl InnerQueue {
    fn get_ready(&mut self) -> Option<JobRealization> {
        for (job, state) in &mut self.jobs {
            if *state == JobState::Ready {
                *state = JobState::Running;
                return Some(job.clone());
            }
        }

        None
    }

    fn all_tasks_distributed(&self) -> bool {
        for (_, state) in &self.jobs {
            if *state != JobState::Finished && *state != JobState::Running {
                return false;
            }
        }

        true
    }
}

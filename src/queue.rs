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
    failed: bool,
}

impl Queue {
    pub fn new() -> Self {
        let inner = InnerQueue {
            jobs: VecDeque::new(),
            states: HashMap::new(),
            done: false,
            failed: false,
        };
        Queue {
            inner: Arc::new(Mutex::new(inner)),
            cond_fetch_job: Arc::new(Condvar::new()),
        }
    }

    pub fn enqueue(&self, job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        if inner.jobs.contains(&job) { return; }
        inner.jobs.push_back(job.clone());
        inner.states.insert(job, JobState::Ready);
        drop(inner);

        self.cond_fetch_job.notify_one();
    }

    pub fn fetch(&self) -> Option<JobRealization> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if (inner.done && !inner.has_alive_tasks()) || inner.failed {
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
        self.cond_fetch_job.notify_all();
    }

    pub fn failed(&self, failed_job: JobRealization) {
        let mut inner = self.inner.lock().unwrap();
        inner.states.insert(failed_job, JobState::Failed);
        inner.failed = true;
        self.cond_fetch_job.notify_all();
    }

    pub fn done(&self) {
        self.inner.lock().unwrap().done = true;
        self.cond_fetch_job.notify_all();
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().jobs.len()
    }
}

impl InnerQueue {
    fn dependencies_satisfied(&self, job: JobRealization) -> bool {
        for dep in job.dependencies() {
            if self.states.get(&dep) != Some(JobState::Finished).as_ref() {
                return false;
            }
        }

        true
    }

    fn get_ready(&mut self) -> Option<JobRealization> {
        let mut ret = None;
        for job in &self.jobs {
            if *self.states.get(job).unwrap() == JobState::Ready
                && self.dependencies_satisfied(job.clone()) {

                ret = Some(job.clone())
            }
        }

        if let Some(job) = &ret {
            self.states.insert(job.clone(), JobState::Running);
            self.jobs.retain(|j| j != job)
        }

        ret
    }

    /// Determines whether the task is running or may be run in the future
    fn task_alive(&self, job: JobRealization) -> bool {
        let state = match self.states.get(&job) {
            Some(state) => *state,
            None => return false,
        };

        if state == JobState::Finished || state == JobState::Failed {
            return false;
        }

        for dep in job.dependencies() {
            if !self.task_alive(dep.clone())
                    && self.states.get(&dep) != Some(JobState::Finished).as_ref() {
                return false;
            }
        }

        true
    }

    fn has_alive_tasks(&self) -> bool {
        for job in &self.jobs {
            if self.task_alive(job.clone()) {
                return true;
            }
        }

        false
    }
}

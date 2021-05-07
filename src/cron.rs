struct Every<T> {
    id: usize,
    t: std::time::Duration,
    running: std::time::Duration,
    callback: Box<dyn FnMut(&mut T) -> ControlFlow>,
}

struct After<T> {
    id: usize,
    t: std::time::Duration,
    running: std::time::Duration,
    callback: Box<dyn FnMut(&mut T) -> ControlFlow>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ControlFlow {
    Continue,
    Stop,
}

impl Into<ControlFlow> for () {
    fn into(self) -> ControlFlow {
        ControlFlow::Continue
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ID(usize);

pub struct Cron<T> {
    index: usize,
    t: std::time::Duration,
    every_callbacks: Vec<Every<T>>,
    after_callbacks: Vec<After<T>>,
}

impl<T> std::default::Default for Cron<T> {
    fn default() -> Self {
        Self {
            index: 0,
            t: Default::default(),
            every_callbacks: vec![],
            after_callbacks: vec![],
        }
    }
}

impl<T> Cron<T> {
    #[allow(unused)]
    pub fn every<C, F>(&mut self, t: std::time::Duration, mut callback: F) -> ID
    where
        C: Into<ControlFlow>,
        F: FnMut(&mut T) -> C + 'static,
    {
        let id = self.new_id();
        self.every_callbacks.push(Every {
            id,
            t,
            running: Default::default(),
            callback: Box::new(move |t: &mut T| (callback)(t).into()),
        });
        ID(id)
    }

    #[allow(unused)]
    pub fn after<C, F>(&mut self, t: std::time::Duration, mut callback: F) -> ID
    where
        C: Into<ControlFlow>,
        F: FnMut(&mut T) -> C + 'static,
    {
        let id = self.new_id();
        self.after_callbacks.push(After {
            id,
            t,
            running: Default::default(),
            callback: Box::new(move |t: &mut T| (callback)(t).into()),
        });
        ID(id)
    }

    #[allow(unused)]
    pub fn remove(&mut self, id: ID) -> bool {
        let id = id.0;
        if let Some(index) = self.every_callbacks.iter().position(|entry| entry.id == id) {
            self.every_callbacks.remove(index);
            return true;
        }
        if let Some(index) = self.after_callbacks.iter().position(|entry| entry.id == id) {
            self.after_callbacks.remove(index);
            return true;
        }
        false
    }

    #[allow(unused)]
    pub fn contains(&self, id: ID) -> bool {
        let id = id.0;
        if self
            .every_callbacks
            .iter()
            .find(|every| every.id == id)
            .is_some()
        {
            true
        } else {
            self.after_callbacks
                .iter()
                .find(|after| after.id == id)
                .is_some()
        }
    }

    fn new_id(&mut self) -> usize {
        let id = self.index;
        self.index += 1;
        id
    }

    // TODO: return invalidated IDs
    pub fn update(&mut self, dt: std::time::Duration, ctx: &mut T) {
        self.t += dt;
        self.after_callbacks.retain(|after| after.running < after.t);

        let mut every_callbacks = vec_mut_scan::VecMutScan::new(&mut self.every_callbacks);
        while let Some(mut every) = every_callbacks.next() {
            every.running += dt;
            if every.running >= every.t {
                every.running = every.running - every.t;
                let cf = (every.callback)(ctx);
                if cf == ControlFlow::Stop {
                    every.remove();
                }
            }
        }

        let mut after_callbacks = vec_mut_scan::VecMutScan::new(&mut self.after_callbacks);
        while let Some(mut after) = after_callbacks.next() {
            after.running += dt;
            if after.running >= after.t {
                (after.callback)(ctx);
                after.remove();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_basic() {
        let time_step = std::time::Duration::from_secs(1);

        let mut ctx = Cron::default();
        let cb = ctx.every(time_step, |counter: &mut i32| {
            *counter += 1;
        });

        let mut counter = 0;
        ctx.update(time_step, &mut counter);
        assert_eq!(counter, 1);
        assert!(ctx.remove(cb));
    }

    #[test]
    fn after_basic() {
        let time_step = std::time::Duration::from_secs(1);

        let mut ctx = Cron::default();
        let cb = ctx.after(time_step, |counter: &mut i32| {
            *counter += 1;
        });

        let mut counter = 0;
        ctx.update(time_step, &mut counter);
        assert_eq!(counter, 1);

        assert!(!ctx.remove(cb));
    }

    #[test]
    fn remove_test() {
        let time_step = std::time::Duration::from_secs(1);
        let mut ctx = Cron::default();
        let cb = ctx.every(time_step, |counter: &mut i32| {
            *counter += 1;
        });
        assert!(ctx.remove(cb));
        assert!(!ctx.remove(cb));
    }

    #[test]
    fn control_flow_test() {
        let time_step = std::time::Duration::from_secs(1);

        let mut ctx = Cron::default();
        let cb1 = ctx.every(time_step, |counter: &mut i32| {
            *counter += 1;
            if *counter >= 3 {
                ControlFlow::Stop
            } else {
                ControlFlow::Continue
            }
        });

        let mut counter = 0;
        while ctx.contains(cb1) {
            ctx.update(time_step, &mut counter);
        }
        assert_eq!(counter, 3);
    }

    // #[test]
    // fn ownership_test() {
    //     struct FunctionHolder<T> {
    //         func: Box<dyn FnMut(&mut T)>
    //     }
    //
    //     let mut x = 1;
    //     let mut f = FunctionHolder {
    //         func: Box::new(move |t: &mut (&mut i32)| {
    //             println!("{}", **t + x);
    //             **t += 1;
    //             x += 1;
    //         })
    //     };
    //
    //     let mut counter = 2;
    //     let mut counter2 = 3;
    //     (f.func)(&mut (&mut counter));
    //     println!("{}", counter);
    // }
}

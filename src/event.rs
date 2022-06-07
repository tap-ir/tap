//! Events let send and receive data trough channel.

use crossbeam::crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Clone, Default)]
pub struct EventChannel<T>
{
  pub registered : Vec<Sender<T>>,
}

impl<T : Clone> EventChannel<T>
{
  pub fn new() -> Self
  {
    EventChannel::<T>{ registered : Vec::new() }
  }
 
  /// Return a new events receiver
  pub fn register(&mut self) -> Events<T> 
  {
    let (sender, receiver) = unbounded();
    self.registered.push(sender);

    Events{ receiver }
  }

  /// Send event
  pub fn update(&self, event : T)
  {
    for handler in self.registered.iter()
    {
      handler.send(event.clone()).unwrap()
    }
  }
}

/**
 *  Events receiver 
 **/
pub struct Events<T>
{
  pub receiver : Receiver<T>,
}

impl<T> Events<T>
{
  pub fn events(&self) -> Vec<T>
  {
    let mut events : Vec<T> = Vec::new();

    while let Ok(event) = self.receiver.try_recv()
    {
      events.push(event);
    };
    events
  }
}

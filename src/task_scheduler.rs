//! The scheduler is in charge of runing tasks ([plugin instancce](PluginInstance) and [argument](PluginArgument)) in differents [workers](Worker).

use std::fmt;
use std::thread;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::error::{RustructError};
use crate::tree::Tree;
use crate::plugin::{PluginInstance, PluginArgument, PluginEnvironment, PluginResult};

use log::info;
use anyhow::{Result, Error};
use crossbeam::crossbeam_channel::{unbounded, bounded, Sender, Receiver};
use serde::{Serialize, Deserialize};
use std::panic::AssertUnwindSafe;

pub type TaskId = u32;
pub type TaskResult = Result<PluginResult, Arc<Error>>;

///Enum indicating state of a plugin (Waiting, Launched, Finished).
#[derive(Debug, Clone)] 
pub enum TaskState
{
  /// Plugin is waiting to be runned
  Waiting(Task), 
  /// Plugin is running
  Launched(Task), //Rename it running
  /// Plugin has finished running
  Finished(Task, TaskResult),
}

/// A [task](Task) is used to run a plugin it's made of a unique `id`, a `plugin_name` and some plugin [`argument`](PluginArgument).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task
{
  /// The unique id of the task
  pub id : TaskId,
  /// The name of the plugin
  pub plugin_name : String,
  /// Argument to the plugin
  pub argument : PluginArgument,
}

impl fmt::Display for Task
{
   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
   {
      write!{f, "({}) {}({})", self.id, self.plugin_name, self.argument} 
   }
}

/// Launch in a thread and used to managed tasks state.Wait to receive a message from Worker and update the task state accordingly.
struct TasksHandler
{
  /// This is used by us to receive the result of task from the workers.
  task_state : Receiver<TaskState>,
  /// Send to task scheduler which task id we updated last.
  task_update : Sender<TaskId>,
  /// This is the map of TaskState that is updated via the pool of worker message.
  tasks : Arc<RwLock<HashMap<TaskId, TaskState>>>,
}

impl TasksHandler
{
  /// Return a new task handler.
  pub fn new(task_state : Receiver<TaskState>, task_update : Sender<TaskId>, tasks : Arc<RwLock<HashMap<TaskId, TaskState>>>) -> Self
  {
    TasksHandler{ task_state, task_update, tasks }
  }

  /// Update the task mask when arrive a new message from the worker pool.
  fn update(&self) 
  {
    //wait blocking for new task
    for task_state in self.task_state.iter()
    {
       let task = match &task_state
       {
         TaskState::Waiting(task) => task, 
         TaskState::Launched(task) => task, 
         TaskState::Finished(task, _) => task, 
       };

       let mut tasks = self.tasks.write().unwrap(); //we don't want to lock the tasks map when waiting on the channel, if we do that before the block the tasks will be locked on write during a potential infinite time
       tasks.insert(task.id, task_state.clone());
       self.task_update.send(task.id).unwrap();
    }
  }
}

/// Boxed PluginInstance. 
type BoxPluginInstance = Box<dyn PluginInstance + Sync + Send>;

/// The scheduler is in charge of running [Task] (plugin [instance](PluginInstance) and [argument](PluginArgument)).
pub struct TaskScheduler
{
  ///This is used to send a new [Task] to a [worker](Worker), to then be executed.
  new_task : Sender<(Task, BoxPluginInstance, Option<Sender<TaskResult>>)>,
  ///Receive update from the [TasksHandler] when the `task` [map](HashMap) is changed.
  task_update : Receiver<TaskId>,
  ///An arc ref to the [TasksHandler] `task` [map](HashMap).
  tasks : Arc<RwLock<HashMap<TaskId, TaskState>>>,
}

/// Provide different method to run, schedule and create new [task](Task).
impl TaskScheduler
{
  /// Instantiate a new scheduler.
  pub fn new(tree : Tree) -> Self
  {
    let (new_task_sender, new_task_receiver) = unbounded();
    let (task_state_sender, task_state_receiver) = unbounded();
    let (task_update_sender, task_update_receiver) = unbounded();

    let tasks = Arc::new(RwLock::new(HashMap::new()));
    let task_handler = TasksHandler::new(task_state_receiver, task_update_sender, tasks.clone());

    TaskScheduler::launch_task_handler(task_handler);
    TaskScheduler::launch_pool(&tree, num_cpus::get(), new_task_receiver, task_state_sender);
    TaskScheduler{ new_task : new_task_sender , task_update : task_update_receiver, tasks }
  }

  fn launch_task_handler(task_handler : TasksHandler) 
  {
    let _ = thread::spawn(move || {task_handler.update();} );
  }

  fn launch_pool(tree : &Tree, thread_count : usize, receiver : Receiver<(Task, BoxPluginInstance, Option<Sender<TaskResult>>)>, task_state_sender : Sender<TaskState>) 
  {  
    for id in  0..thread_count
    {
      let worker = Worker::new(id, tree.clone(), receiver.clone(), task_state_sender.clone());

      let _ = thread::spawn(move || 
      {
        worker.run();
      });
    }
  }

  /// Create a new [task](Task) and add it to the the tasks list, if a waiter is present we will send it a message when the task is finished.
  fn push(&self, plugin: Box<dyn PluginInstance + Sync + Send>, argument : PluginArgument, relaunch : bool, waiter : Option<Sender<TaskResult>>) -> Result<TaskId, Error>
  {
    if relaunch || !self.exist(plugin.name(), &argument)
    {
      let mut tasks = self.tasks.write().unwrap();
      let task_id = tasks.len() + 1;
      let task = Task{ plugin_name : plugin.name().to_string(), argument, id : task_id as u32 };
      //XXX rather send a message to thread so it update the state herself ?
      tasks.insert(task_id as u32, TaskState::Waiting(task.clone()));

      //send new task to the pool
      self.new_task.send((task, plugin, waiter)).unwrap();
      Ok(task_id as u32)
    } else {
      Err(RustructError::PluginAlreadyRunned.into())
    }
  }

  /// Create a new task and schedule it to be launched, return a task id or an error if task already exist.
  pub fn schedule(&self, plugin: Box<dyn PluginInstance + Sync + Send>, argument : PluginArgument, relaunch : bool) -> Result<TaskId, Error>
  {
    self.push(plugin, argument, relaunch, None)
  }

  /// Create a new [task](Task) and block until the [task](Task) is finished, return a [plugin result](PluginResult) or an error, if [task](Task) exist or if execution of the [task](Task) failed.
  pub fn run(&self, plugin : Box<dyn PluginInstance + Sync + Send>, argument : PluginArgument, relaunch : bool) -> Result<PluginResult, Arc<Error>>
  {
    let (sender, receiver) = bounded(1);
    let result = self.push(plugin, argument, relaunch, Some(sender));
    
    match result
    {
      Ok(_id) => receiver.recv().unwrap(),
      Err(err) => Err(Arc::new(err)), //send it as a module error but it's a TaskSched error
    }
  }

  /// Check if all [task](Task) in the `tasks` [map](HashMap) are finished.
  pub fn tasks_are_finished(&self) -> bool
  {
    let tasks = self.tasks.read().unwrap();
    for task in tasks.values()
    {
      match task
      {
        TaskState::Waiting(_) => return false,
        TaskState::Launched(_) => return false,
        TaskState::Finished(_, _) => (),
      }
    }
    true 
  }

  /// Wait until all scheduled [task](Task) are finished.
  // if an other thread add task to the scheduler, a thread could wait for task to join
  // be will be to have a join([task_id]) so we sure we wait only on our created tasks 
  pub fn join(&self) 
  {
    if self.tasks_are_finished()
    {
      return 
    }

    for _ in self.task_update.iter()
    {
      //match if task is finished we can check if all are finished
      if self.tasks_are_finished()
      {
        break
      }
    }
  }

  /// Return a [TaskState] corresponding to a task id.
  pub fn task(&self, id : TaskId) -> Option<TaskState>
  {
    self.tasks.read().unwrap().get(&id).cloned()
  }

  /// Return a vec of [TaskState] for corresponding task id.
  pub fn tasks(&self, ids : Vec<TaskId>) -> Vec<TaskState>
  {
    let tasks = self.tasks.read().unwrap();
    ids.iter().filter_map(|id| tasks.get(id).cloned()).collect()
  }

  /// Return a copy of all the [task state](TaskState) for all [task](Task) in the `tasks` map.
  pub fn to_vec(&self) -> Vec<TaskState>
  {
    self.tasks.read().unwrap().values().cloned().collect()  
  }

  /// Return the current count of [tasks](TaskState) added to the [scheduler](TaskScheduler).
  pub fn task_count(&self) -> u32
  {
    self.tasks.read().unwrap().len() as u32
  }

  /// Return all finished [task](TaskState) and their [result](TaskResult).
  pub fn tasks_finished(&self) -> Vec<(Task, TaskResult)>
  {
     self.tasks.read().unwrap().values().filter_map(|task| match task { TaskState::Finished(task, res) => Some((task.clone(), res.clone())), _ => None} ).collect()
  }

  /// Check if a task with for same plugin and argument was already added to the scheduler.
  /// That's used to avoid relaunching same task twice.
  fn exist(&self, plugin_name : &str, argument : &str) -> bool
  {
    for task_state in self.tasks.read().unwrap().values()
    {
      match task_state
      {
        TaskState::Waiting(task) | TaskState::Launched(task) | TaskState::Finished(task, _) =>
        {
          if plugin_name == task.plugin_name && argument == task.argument
          {
            return true
          }
        }
      }
    }
    false
  }
}

/**
 * A worker for running a [plugin instance](PluginInstance).
 **/
pub struct Worker
{
  /// Worker unique id.
  id : usize,
  /// Reference to the TAP Tree.
  tree : Tree,
  /// Receive new Task to execute on that channel.
  receiver : Receiver<(Task, BoxPluginInstance, Option<Sender<TaskResult>>)>,
  /// Send result of a Task on that channel.
  sender : Sender<TaskState>,
}

impl Worker
{
  /// Return a new [Worker].
  fn new(id : usize, tree : Tree, receiver : Receiver<(Task, BoxPluginInstance, Option<Sender<TaskResult>>)>, sender : Sender<TaskState>) -> Self
  {
    Worker{id, tree, receiver, sender}
  }

  fn find_task(&self) -> (Task, BoxPluginInstance, Option<Sender<TaskResult>>)
  {
     loop
     {
       if let Ok(task) = self.receiver.recv()
       {
          return task;
       }
     }
  }

  /// Loop and wait to receive a new task through the `receiver` channel then execute the plugin and send it's return value (result) via the `sender` channel.
  fn run(&self)
  {
    loop
    {
      let (task, mut plugin_instance, waiter) = self.find_task();
      self.sender.send(TaskState::Launched(task.clone())).unwrap();
      info!("task runned : {}({}) {} on worker {}", task.plugin_name, task.id, task.argument, self.id);

      //add nodes to tree here if tree is not passed to modules
      let environment = PluginEnvironment::new(self.tree.clone(), Some(self.sender.clone()));
      //pass sender to modules to update state with more info ? 

      //we catch unwindable panic in thread running plugin assuming no use of unsafe code
      let panic = std::panic::catch_unwind(AssertUnwindSafe(|| 
      {
        plugin_instance.run(task.argument.clone(), environment)
      }));

      let result = match panic
      {
        Ok(result) => result,
        Err(err) => Err(anyhow::anyhow!("Error thread of task {}({}) {} panicked : {:?}", task.plugin_name, task.id, task.argument, err))
      };

      let result = match result
      {
        Ok(result) => 
        { 
          info!("task finished : {}({})", task.plugin_name, task.id);
          Ok(result) 
        },
         //store as string and display error here ?
        Err(error) => 
        { 
           info!("task finished  : {}({}) with error {} ", task.plugin_name, task.id, error);
           Err(Arc::new(error)) } ,      
        };
      
      //info!("task finished : {}({}) {:?}", task.plugin_name, task.id);
      //info!("result for task : {}({}) {:?}", task.plugin_name, task.id, result);
      if let Some(waiter) = waiter
      {
        waiter.send(result.clone()).unwrap()
      }
      let finished_task = TaskState::Finished(task, result);
      self.sender.send(finished_task.clone()).unwrap(); //update task map
    }
  }
}

#[cfg(test)]
mod tests
{
    use super::TaskScheduler;
    use crate::plugin::PluginInfo;
    use crate::plugin_dummy;
    use crate::tree::Tree;

    use serde_json::json;

    #[test]
    fn schedule_plugins_join_get_results()
    {
       let tree = Tree::new();
       let root_id = tree.root_id;
       let scheduler = TaskScheduler::new(tree);
       let mut task_ids = Vec::new();

       let plugin_info = plugin_dummy::Plugin::new();
       for _ in 0..24
       {
          let plugin = plugin_info.instantiate();
          let arg = json!({ "parent" : Some(root_id), "file_name" : "/home/user/test.txt", "offset" : 0});
          if let Ok(id) = scheduler.schedule(plugin, arg.to_string(), false)
          {
            task_ids.push(id);
          }
       }
       scheduler.join();

       for _result in scheduler.tasks(task_ids) 
       {
         () //we launch the same plugins 24 times, so must return result with error
       }
    }
}

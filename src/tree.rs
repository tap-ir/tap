//! The tree let you access all the node and their attributes created by the different plugins, 
//! in an uniform and reflective ways.

use std::fmt;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use crate::value::Value;
use crate::node::Node;

use indextree::{Arena, NodeId};
use serde::{Serialize, Deserialize};
use serde::ser::{Serializer, SerializeMap};
use schemars::{JsonSchema};

pub type TreeNodeId = NodeId;
pub type TreeNode = Arc<Node>;
pub type TreeArena = Arena<TreeNode>;
pub type TreeLock = RwLock<TreeArena>;
pub type TreeArc = Arc<RwLock<TreeArena>>;

#[derive(Serialize, Deserialize)]
pub struct ChildInfo
{
  pub name : String,
  pub id : TreeNodeId,
  pub has_children : bool,
}

#[derive(JsonSchema)]
#[serde(remote = "TreeNodeId")]
pub struct TreeNodeIdSchema
{
  pub index1 : usize,
  pub stamp : u16,
}

#[derive(JsonSchema)]
#[serde(remote = "VecTreeNodeId")]
pub struct VecTreeNodeIdSchema
{
  pub ids : Vec<TreeNodeIdSchema>,
}

/**
 * One of the main structure of TAP.
 * Tt contain nodes, that contain [attribute](crate::attribute::Attribute) with [value](Value) of different type.
 * New nodes are generally created by the different parser, then nodes and attributes contained in the tree can be accessed via various method.
 */
#[derive(Clone)]
pub struct Tree
{
  tree : TreeArc,
  pub root_id : TreeNodeId,
}

impl Tree
{
   /// Create the tree and the root node.
  pub fn new() -> Self
  {
    let mut tree = Arena::new();
    let root_node = Arc::new(Node::new("root"));
    let root_id = tree.new_node(root_node);
    Tree{ tree : Arc::new(RwLock::new(tree)), root_id } 
  }

  /// Return the underlying [tree arena](TreeArena).
  pub fn arena(&self) -> RwLockReadGuard<TreeArena>
  {
    self.tree.read().unwrap()
  }

  /// Create a new [`node`](Node) in the [tree](Tree) and return corresponding [id](TreeNodeId).
  pub fn new_node(&self, node : Node) -> TreeNodeId
  {
    let mut tree = self.tree.write().unwrap();
    tree.new_node(Arc::new(node))
  }

  /// Add a node via it's [`node_id`](TreeNodeId) as child of the [`parent_id`](TreeNodeId) [node](Node).
  pub fn add_child_from_id(&self, parent_id : NodeId, node_id : NodeId)
  {
    let mut tree = self.tree.write().unwrap();
    parent_id.append(node_id, &mut tree);
  }

  /// Create a new [TreeNodeId] for [`node`](Node), add it as child of `parent_id` and return the new [node id](TreeNodeId.)
  pub fn add_child(&self, parent_id : NodeId, node : Node) -> anyhow::Result<TreeNodeId>
  {
    let mut tree = self.tree.write().unwrap();
    //this is very slow
    //for child_id in parent_id.children(&tree) //check for same name
    //{
      //if tree[child_id].get().name() == node.name() //don't use []
      //{
        //return None;
      //}
    //}

    let node_id = tree.new_node(Arc::new(node));
    parent_id.append(node_id, &mut tree);
    //if event registered ? avoid to have a big queue ? 
    //self.node_event.update(node_id); //XXX ? 
    Ok(node_id)
  }

  /// Return [node id](TreeNodeId) of the parent of the [node](Node).
  pub fn parent_id(&self, node_id : NodeId) -> Option<NodeId>
  {
     let tree = self.tree.read().unwrap();
     tree[node_id].parent()
  }

  /// Return the children of the provided NodeId as a Vector of NodeId.
  pub fn children_id(&self, node_id : NodeId) -> Vec<NodeId>
  {
    let mut ids = Vec::new();
    let tree = self.tree.read().unwrap();

    //what happen if node_id is deserialized and didn't exist ?
    for child_id in node_id.children(&tree)//collect 
    {
      ids.push(child_id)
    }
    ids
  }

  /// Return the children of the provided NodeId as a Vector of Node.
  pub fn children(&self, node_id : NodeId) -> Vec<Arc<Node>>
  {
    let mut nodes = Vec::new();
    let tree = self.tree.read().unwrap();

    for child_id in node_id.children(&tree) 
    {
      nodes.push(tree[child_id].get().clone())//collect //XXX check id don't use []
    }
    nodes 
  }

  /// Return children from a node `root` path recusively as a [Vec]<[TreeNodeId]>.
  #[inline]
  pub fn children_rec(&self, root : Option<&str>) -> Option<Vec<TreeNodeId>>
  {
    let arena = self.arena();

    let root_id = match root
    {
      Some(root) => self.get_node_id(root)?,
      None => self.root_id,
    };
    Some(root_id.descendants(&arena).collect())
  }

  /// Return the name of the children for `node_id`. 
  pub fn children_name(&self, node_id : NodeId) -> Vec<String>
  {
    let mut names = Vec::new();
    let tree = self.tree.read().unwrap();

    for child_id in node_id.children(&tree)
    {
      names.push(tree[child_id].get().name())//collect //XXX check id don't use []
    }
    names
  }

  /// Check if [node](Node) as children.
  pub fn has_children(&self, node_id: NodeId) -> bool
  {
    let tree = self.tree.read().unwrap();
    tree[node_id].first_child().is_some()
  }

  /// Return different info for all children of a [node](Node).
  pub fn children_id_name(&self, node_id : NodeId) -> Vec<ChildInfo>
  {
     let mut infos = Vec::new();
     let tree = self.tree.read().unwrap();

     for child_id in node_id.children(&tree)
     {
        //XXX really usefull for child ? to display tree or as n+1 ?
        //node already serialize it 
        let has_children = tree[child_id].first_child().is_some(); 
        let name = tree[child_id].get().name();
        let id = child_id;
        infos.push(ChildInfo{ name, id, has_children })
     }
     //we sort child by name insenstive to case before returning the list
     infos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
     infos
  }

  /// Return a path from a [node id](TreeNodeId).
  pub fn node_path(&self, node_id : TreeNodeId) -> Option<String>
  {
    let mut path : String = String::new();

    let tree = self.tree.read().unwrap();
    for next_node_id in node_id.ancestors(&tree)
    {
      let next_node = match tree.get(next_node_id)
      {
        Some(next_node) => next_node,
        None => return None,
      };
      if next_node.is_removed() 
      {
        return None;
      }
      path = next_node.get().name().to_owned() + "/" + &path;
    }
    Some("/".to_owned() + &path[..path.len()-1])
  }

  /// Return a [node](TreeNode) from a [node id](NodeId).
  pub fn get_node_from_id(&self, node_id : TreeNodeId) -> Option<TreeNode> 
  {
    let tree = self.tree.read().unwrap();
    if let Some(tree_node) = tree.get(node_id)
    {
      if tree_node.is_removed() //this is needed if the remove function is used, but can be slower 
      {
        return None;
      }
      return Some(tree_node.get().clone())
    }
    None
  }

  /// Remove node and descendants from the tree.
  pub fn remove(&self, node_id : NodeId) 
  {
     let mut tree = self.tree.write().unwrap();
     //XXX 
     //Please note that the node will not be removed from the internal arena storage, but marked as removed. Traversing the arena returns a plain iterator and contains removed elements too.
     //Node count will still be the same
     node_id.remove_subtree(&mut tree);
  }

  /// Return a [node](TreeNode) from a path.
  pub fn get_node(&self, path : &str) -> Option<TreeNode>
  {
    self.get_node_id(path).map(|node_id| self.get_node_from_id(node_id).unwrap()) //XXX fix unwrap
  }

  //put in query, so we can used more advanced search
  ///Search recursively for nodes matching `path`, starting from the root `from_id`.
  pub fn find_node_from_id(&self, from_id : TreeNodeId, path : &str) -> Option<TreeNodeId>
  {
    let mut pathes = path.split('/').collect::<Vec<&str>>();

    if pathes.is_empty()
    {
      return None;
    }

    if pathes[0].is_empty()
    {
      pathes.remove(0);
    }

    if pathes.is_empty()
    {
      return None;
    }

    if pathes[pathes.len()-1].is_empty()
    {
      pathes.remove(pathes.len()-1);
    }

    let mut found;
    let mut current_node_id = from_id;

    let tree = self.tree.read().unwrap();
    for path in pathes.into_iter()
    {
      found = false;
      for child_id in current_node_id.children(&tree)
      {
         let node = &tree[child_id].get();
         if path == node.name()
         {
            found = true;
            current_node_id = child_id;
            break;
         }
      }
      if !found
      {
        return None
      }
    }
    Some(current_node_id)
  }

  /// Return a [node id](TreeNodeId) from node `path`.
  pub fn get_node_id(&self, pathes : &str) -> Option<TreeNodeId>
  {
    let mut pathes = pathes.split('/').collect::<Vec<&str>>();

    //path is empty after split
    if pathes.is_empty()
    {
      return None; 
    }

    //if path start with / or is "" , path[0] == ""
    if pathes[0].is_empty() 
    {
      pathes.remove(0);
    }

    //if path is "" path len() is now == 0
    if pathes.is_empty()
    {
      return None;
    }

    //if path has / at last index will be "" 
    if pathes[pathes.len()-1].is_empty()
    {
      pathes.remove(pathes.len()-1);
    }

    //now path[0] should contain "root" in any cases
    if pathes[0] != "root"
    {
      return None;
    }
    //now path should start by root or 
    if pathes.len() == 1
    {
      return Some(self.root_id);
    }

    let mut found;
    let mut current_node_id = self.root_id;

    let tree = self.tree.read().unwrap();
    for path in pathes.into_iter().skip(1) //path[0] == "root", we skip it
    {
      found = false;
      for child_id in current_node_id.children(&tree)
      {
        let node = &tree[child_id].get(); //don't use [] XXX
        if path == node.name() 
        {
           found = true;
           current_node_id = child_id;
           break;
        }
      } 
      if !found
      {
        return None
      }
    }
    Some(current_node_id)
  }

  /// Return number of [nodes](TreeNode) in the tree.
  pub fn count(&self) -> usize
  {
    self.tree.read().unwrap().count()
  }
}

impl Default for Tree
{
  fn default() -> Self
  {
    Self::new()
  }
}

impl fmt::Display for Tree 
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
  {
    let tree = self.tree.read().unwrap();
    for node in self.root_id.descendants(&tree)
    {
      writeln!(f, "{} : {}", self.node_path(node).unwrap(),  tree[node].get() as &Node).unwrap();
    }
    Ok(())
  }
}

impl Serialize for Tree
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer,
  {
     let tree = self.tree.read().unwrap();
     let mut map = serializer.serialize_map(Some(tree.count()))?;

     for attribute_id in self.root_id.descendants(&tree)
     {
       let attribute = &tree[attribute_id].get();
       map.serialize_entry(&attribute.name(), &attribute.value())?;
     }
     map.end()
  }
}

/**
 *  AttributePath is an easy way to get any kind of node value, even trait object, via serialization.
 */
#[derive(Debug, Serialize, Deserialize,Clone, PartialEq,JsonSchema)]
pub struct AttributePath
{
  #[schemars(with = "TreeNodeIdSchema")] 
  pub node_id : TreeNodeId,
  pub attribute_name : String,
}

impl AttributePath 
{
   /// Create an attribute path object using the convenient syntax node_path:attribute_name.
   // XXX put as method of tree ? tree.new_attribute_path ?
   pub fn new(tree : &Tree, path : &str) -> Option<AttributePath>
   {
     let splitted : Vec<&str> = path.split(':').collect();
      
     match splitted.len()
     {
       2 =>  tree.get_node_id(splitted[0]).map(|node_id| AttributePath{node_id, attribute_name : splitted[1].to_string()}),
       _ => None,
     }
  }

  /// Return the [node][TreeNode] related to the [attribute](crate::attribute::Attribute).
  pub fn get_node(&self, tree : &Tree) -> Option<TreeNode>
  {
    tree.get_node_from_id(self.node_id) //useful 
  }

  /// Return the [value](Value) of the [attribute](crate::attribute::Attribute)
  pub fn get_value(&self, tree : &Tree) -> Option<Value>
  {
    let node = tree.get_node_from_id(self.node_id)?;
    node.value().get_value(&self.attribute_name) //get_value must resolved '.' notation
  }
}

//test tree
#[cfg(test)]
mod tests
{
  use super::{Tree, AttributePath}; 
  use crate::node::Node;
  use crate::value::Value;

  #[test]
  fn create_tree_and_get_root()
  {
    let tree = Tree::new();
    assert!(tree.node_path(tree.root_id).unwrap() == "/root");
  }

  #[test]
  fn create_tree_with_node()
  {
    let tree = Tree::new();
    let root_id = tree.root_id;

    let test_node_id_1 = tree.add_child(root_id, Node::new("test1")).unwrap();
    let test_node_id_2 = tree.add_child(root_id, Node::new("test2")).unwrap();
    let test_node_id_3 = tree.add_child(root_id, Node::new("test3")).unwrap();
     
    let child_node_id_1= tree.add_child(test_node_id_1, Node::new("child1")).unwrap(); 
    tree.add_child(test_node_id_2, Node::new("child2")).unwrap(); 
    tree.add_child(test_node_id_3, Node::new("child3")).unwrap(); 

    let _sub_child_node_id_1= tree.add_child(child_node_id_1, Node::new("subchild1")).unwrap(); 
    let _sub_child_node_id_2 = tree.add_child(child_node_id_1, Node::new("subchild2")).unwrap(); 
    let _sub_child_node_id_3 = tree.add_child(child_node_id_1, Node::new("subchild3")).unwrap(); 

    println!("{}", tree);
    /*println!("{}", tree.node_path(sub_child_node_id_1));
    assert!(tree.node_path(sub_child_node_id_1) == "/root/test1/child1/subchild1");
    assert!(tree.node_path(sub_child_node_id_3) == "/root/test1/child1/subchild3");

    let _id = tree.get_node_id(root_id, "/root").unwrap(); 
    println!("{}", tree.node_path(tree.get_node_id(root_id, "/root/test1").unwrap()));
    assert!("/root/test1/child1/subchild1" == tree.node_path(tree.get_node_id(root_id, "/root/test1/child1/subchild1").unwrap()));
    assert!(sub_child_node_id_1 == tree.get_node_id(root_id, "/root/test1/child1/subchild1").unwrap());
    assert!(sub_child_node_id_1 == tree.get_node_id(root_id, "/root/test1/child1/subchild1").unwrap());
    assert!(sub_child_node_id_3 == tree.get_node_id(root_id, "/root/test1/child1/subchild3").unwrap());*/
  }

  #[test]
  fn get_value_from_attribute_path()
  {
    let tree = Tree::new();
    
    let test_node_id = tree.add_child(tree.root_id, Node::new("test1")).unwrap();

    let child_node = Node::new("child1");
    child_node.value().add_attribute("attribute", Value::U32(0x1000), Some("test attribute"));

    let child_node_id = tree.add_child(test_node_id, child_node).unwrap();

    let attribute_path = AttributePath{ node_id : child_node_id, attribute_name : String::from("attribute")};
    assert!(attribute_path.get_node(&tree).unwrap().name() == "child1");
    assert!(attribute_path.get_value(&tree).unwrap().as_u32() == 0x1000);
  }
}

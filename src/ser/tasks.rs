// Copyright (C) 2018-2022 Daniel Mueller (deso@posteo.net)
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;

use crate::ser::id::Id as IdT;
use crate::ser::tags::Tag;
use crate::ser::tags::Templates;


#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct T(());

/// A serializable and deserializable task ID.
///
/// Note that tasks only have an ID when saved (i.e., in serialized
/// form). In terms of in-memory representation, this ID corresponds
/// most closely to a `db::Id`.
pub type Id = IdT<T>;


/// A task that can be serialized and deserialized.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Task {
  pub summary: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tags: Vec<Tag>,
}

#[cfg(any(test, feature = "test"))]
impl Task {
  /// Create a new task with the given summary and no tags.
  pub fn new<S>(summary: S) -> Self
  where
    S: Into<String>,
  {
    Self {
      summary: summary.into(),
      tags: Default::default(),
    }
  }

  /// A convenience helper for setting the task's tags.
  pub fn with_tags<I>(mut self, tags: I) -> Self
  where
    I: IntoIterator<Item = Tag>,
  {
    self.tags = tags.into_iter().collect();
    self
  }
}


/// Meta data for tasks.
#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TasksMeta {
  #[serde(default)]
  pub templates: Templates,
  /// IDs of tasks in the intended order.
  pub ids: Vec<Id>,
}


/// A struct comprising a list of tasks.
#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Tasks(pub Vec<Task>);


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  use crate::ser::tags::Id as TagId;


  #[test]
  fn serialize_deserialize_task_without_tags() {
    let task = Task::new("task without tags");
    let serialized = to_json(&task).unwrap();
    let deserialized = from_json::<Task>(&serialized).unwrap();

    assert_eq!(deserialized, task);
  }

  #[test]
  fn serialize_deserialize_task() {
    let tags = [
      Tag {
        id: TagId::try_from(2).unwrap(),
      },
      Tag {
        id: TagId::try_from(4).unwrap(),
      },
    ];
    let task = Task::new("this is a task").with_tags(tags);
    let serialized = to_json(&task).unwrap();
    let deserialized = from_json::<Task>(&serialized).unwrap();

    assert_eq!(deserialized, task);
  }

  #[test]
  fn serialize_deserialize_tasks() {
    let task_vec = vec![
      Task::new("task 1").with_tags([
        Tag {
          id: TagId::try_from(10000).unwrap(),
        },
        Tag {
          id: TagId::try_from(5).unwrap(),
        },
      ]),
      Task::new("task 2").with_tags([
        Tag {
          id: TagId::try_from(5).unwrap(),
        },
        Tag {
          id: TagId::try_from(6).unwrap(),
        },
      ]),
    ];
    let tasks = Tasks(task_vec);
    let serialized = to_json(&tasks).unwrap();
    let deserialized = from_json::<Tasks>(&serialized).unwrap();

    assert_eq!(deserialized, tasks);
  }
}

// task_list_box.rs

// *************************************************************************
// * Copyright (C) 2018 Daniel Mueller (deso@posteo.net)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

use std::cell::RefCell;
use std::cmp::max;
use std::cmp::min;
use std::isize;
use std::rc::Rc;

use gui::Cap;
use gui::Event;
use gui::Handleable;
use gui::Id;
use gui::Key;
use gui::MetaEvent;

use event::EventUpdated;
use in_out::InOut;
use query::Query;
use tasks::Id as TaskId;
use tasks::Task;
use tasks::Tasks;
use termui::TermUiEvent;


/// Sanitize a selection index.
fn sanitize_selection(selection: isize, count: usize) -> usize {
  max(0, min(count as isize - 1, selection)) as usize
}


/// A widget representing a list of `Task` objects.
#[derive(Debug, GuiWidget)]
pub struct TaskListBox {
  id: Id,
  tasks: Rc<RefCell<Tasks>>,
  query: Query,
  selection: usize,
  editing: Option<Task>,
}

impl TaskListBox {
  /// Create a new `TaskListBox` widget.
  pub fn new(id: Id, tasks: Rc<RefCell<Tasks>>, query: Query) -> Self {
    TaskListBox {
      id: id,
      tasks: tasks,
      query: query,
      selection: 0,
      editing: None,
    }
  }

  /// Select a task and emit an event indicating success/failure.
  ///
  /// This method takes care of correctly selecting a task after it was
  /// added or modified. After a such an operation task may or may not
  /// be covered by our query anymore, meaning we either need to select
  /// it or find somebody else who displays it and ask him to select it.
  ///
  /// Note that the emitted event chain will only contain an `Updated`
  /// event if the selection changed. However, there may be other
  /// conditions under which an update must happen, e.g., when the
  /// summary or the tags of the task changed. Handling of updates in
  /// those cases is left to clients.
  fn handle_select_task(&mut self, task_id: TaskId, widget_id: Option<Id>) -> Option<MetaEvent> {
    let idx = self.query.position(|x| x.id() == task_id);
    if let Some(idx) = idx {
      let update = self.set_select(idx as isize);
      let event = TermUiEvent::SelectedTask(self.id);
      // Indicate to the parent that we selected the task in
      // question successfully. The widget should make sure to focus
      // us subsequently.
      Some(Event::Custom(Box::new(event))).maybe_update(update)
    } else {
      // We don't have a task with that Id. Bounce the event back to
      // the parent to let it check with another widget.
      let data = Box::new(TermUiEvent::SelectTask(task_id, widget_id));
      let event = Event::Custom(data);
      Some(event.into())
    }
  }

  /// Handle a custom event.
  fn handle_custom_event(&mut self, event: Box<TermUiEvent>) -> Option<MetaEvent> {
    match *event {
      TermUiEvent::SelectTask(task_id, widget_id) => {
        self.handle_select_task(task_id, widget_id)
      },
      TermUiEvent::EnteredText(text) => {
        if let Some(mut task) = self.editing.take() {
          let id = task.id();
          task.summary = text;
          self.tasks.borrow_mut().update(task);
          self.handle_select_task(id, None).update()
        } else if !text.is_empty() {
          let tags = if !self.query.is_empty() {
            let mut task = self.selected_task();
            if task.is_complete() {
              task.toggle_complete()
            }
            let tags = task.tags().cloned().collect();
            tags
          } else {
            Default::default()
          };

          let id = self.tasks.borrow_mut().add(text, tags);
          self.handle_select_task(id, None)
        } else {
          None
        }
      },
      _ => Some(Event::Custom(event).into()),
    }
  }

  /// Retrieve the query associated with this widget.
  pub fn query(&self) -> Query {
    self.query.clone()
  }

  /// Retrieve the current selection index.
  ///
  /// The selection index indicates the currently selected task.
  pub fn selection(&self) -> usize {
    self.selection
  }

  /// Change the currently selected task.
  fn set_select(&mut self, new_selection: isize) -> bool {
    let count = self.query().count();
    let old_selection = self.selection;
    self.selection = sanitize_selection(new_selection, count);

    self.selection != old_selection
  }

  /// Change the currently selected task.
  fn select(&mut self, change: isize) -> bool {
    let new_selection = self.selection as isize + change;
    self.set_select(new_selection)
  }

  /// Retrieve a copy of the selected task.
  ///
  /// This method must only be called if tasks are available.
  fn selected_task(&self) -> Task {
    debug_assert!(!self.query().is_empty());
    // We maintain the invariant that the selection is always valid,
    // which means that we should always expect a task to be found.
    self.query().nth(self.selection).unwrap()
  }
}

impl Handleable for TaskListBox {
  /// Check for new input and react to it.
  fn handle(&mut self, event: Event, _cap: &mut Cap) -> Option<MetaEvent> {
    match event {
      Event::KeyDown(key) |
      Event::KeyUp(key) => {
        match key {
          Key::Char(' ') => {
            let mut task = self.selected_task();
            let id = task.id();
            task.toggle_complete();
            self.tasks.borrow_mut().update(task);
            self.handle_select_task(id, None).update()
          },
          Key::Char('a') => {
            let event = TermUiEvent::SetInOut(InOut::Input("".to_string(), 0));
            let event = Event::Custom(Box::new(event));
            Some(event.into())
          },
          Key::Char('d') => {
            if !self.query().is_empty() {
              let id = self.selected_task().id();
              self.tasks.borrow_mut().remove(id);
              self.select(-1);
              (None as Option<Event>).update()
            } else {
              None
            }
          },
          Key::Char('e') => {
            let task = self.selected_task();
            let string = task.summary.clone();
            let idx = string.len();
            let event = TermUiEvent::SetInOut(InOut::Input(string, idx));
            let event = Event::Custom(Box::new(event));

            self.editing = Some(task);
            Some(event.into())
          },
          Key::Char('J') => {
            if !self.query().is_empty() {
              let to_move = self.selected_task();
              let other = self.query().nth(self.selection + 1);
              if let Some(other) = other {
                self.tasks.borrow_mut().move_after(to_move.id(), other.id());
                self.select(1);
                (None as Option<Event>).update()
              } else {
                None
              }
            } else {
              None
            }
          },
          Key::Char('K') => {
            if !self.query().is_empty() && self.selection > 0 {
              let to_move = self.selected_task();
              let other = self.query().nth(self.selection - 1);
              if let Some(other) = other {
                self.tasks.borrow_mut().move_before(to_move.id(), other.id());
                self.select(-1);
                (None as Option<Event>).update()
              } else {
                None
              }
            } else {
              None
            }
          },
          Key::Char('g') => (None as Option<Event>).maybe_update(self.set_select(0)),
          Key::Char('G') => (None as Option<Event>).maybe_update(self.set_select(isize::MAX)),
          Key::Char('j') => (None as Option<Event>).maybe_update(self.select(1)),
          Key::Char('k') => (None as Option<Event>).maybe_update(self.select(-1)),
          _ => Some(event.into()),
        }
      },
      Event::Custom(data) => {
        match data.downcast::<TermUiEvent>() {
          Ok(e) => self.handle_custom_event(e),
          Err(e) => panic!("Received unexpected custom event: {:?}", e),
        }
      },
    }
  }
}

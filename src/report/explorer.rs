use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use itertools::Itertools;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Spacing},
    prelude::Backend,
    style::{Color, Modifier, Style},
    symbols::merge::MergeStrategy,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Padding, Paragraph},
};
use std::collections::{HashMap, HashSet};
use std::io;

use crate::{
    analysis::dominator_tree::DominatorTree,
    graph::v8_heap_graph::{NodeType, V8HeapGraph},
    report::minimal_node_repr,
    types::NodeId,
    utils::format_bytes,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum UiTreeId {
    /// This is an artificial group node
    Group(usize),

    /// This is a heap node
    Heap(NodeId),
}

impl Default for UiTreeId {
    fn default() -> Self {
        UiTreeId::Group(usize::MAX)
    }
}

#[derive(Clone, Default)]
struct UiTreeNode {
    id: UiTreeId,
    label: String,
    retained_size: usize,
    children: Vec<UiTreeNode>,
}

struct ExplorerState<'a> {
    pub selected: usize,
    pub scroll_offset: usize,
    pub height: usize,
    pub expanded: HashSet<UiTreeId>,
    pub flat_list: Vec<(&'a UiTreeNode, usize)>,
    pub root: &'a UiTreeNode,
    pub info_open: bool,
}

impl<'a> ExplorerState<'a> {
    pub fn new(root: &'a UiTreeNode) -> Self {
        let mut expanded = HashSet::<UiTreeId>::new();
        expanded.insert(UiTreeId::Heap(0)); // Root starts expanded

        let flat_list = flatten_tree(&root, &expanded);

        ExplorerState {
            selected: 0,
            scroll_offset: 0,
            height: 0,
            expanded,
            flat_list,
            root,
            info_open: false,
        }
    }

    pub fn set_selection(&mut self, selected: usize) {
        self.selected = selected;
        if self.selected >= self.scroll_offset + self.height {
            self.scroll_offset = self.selected - self.height + 1;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if delta > 0 {
            self.set_selection((self.selected + delta as usize).min(self.flat_list.len() - 1));
        } else {
            self.set_selection(self.selected.saturating_sub((-delta) as usize));
        }
    }

    pub fn toggle_selected(&mut self) {
        let node_id = self.selected_id();
        if !self.flat_list[self.selected].0.children.is_empty() {
            if self.expanded.contains(&node_id) {
                self.expanded.remove(&node_id);
            } else {
                self.expanded.insert(node_id);
            }
            self.update_flat_list();
        }
    }

    pub fn expand_selected(&mut self) {
        let node_id = self.selected_id();
        if !self.flat_list[self.selected].0.children.is_empty() && !self.expanded.contains(&node_id)
        {
            self.expanded.insert(node_id);
            self.update_flat_list();
        }
    }

    pub fn collapse_selected(&mut self) {
        let node_id = self.selected_id();
        if self.expanded.contains(&node_id) {
            self.expanded.remove(&node_id);
            self.update_flat_list();
        } else {
            // Find parent and collapse it
            let current_depth = self.flat_list[self.selected].1;
            if current_depth > 0 {
                for i in (0..self.selected).rev() {
                    if self.flat_list[i].1 < current_depth {
                        let parent_id = self.flat_list[i].0.id;
                        if self.expanded.contains(&parent_id) {
                            self.expanded.remove(&parent_id);
                            self.update_flat_list();
                            self.set_selection(i);
                        }
                        break;
                    }
                }
            }
        }
    }

    fn selected_id(&self) -> UiTreeId {
        self.flat_list[self.selected].0.id
    }

    fn update_flat_list(&mut self) {
        self.flat_list = flatten_tree(&self.root, &self.expanded);
    }
}

pub fn explore_graph(tree: &DominatorTree, graph: &V8HeapGraph) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Shared state between draw and poll
    let mut root = build_ui_tree(0, tree, graph);
    find_groups_in_ui_tree(&mut root);
    let mut state = ExplorerState::new(&root);

    loop {
        draw(&mut terminal, &mut state)?;
        let action = handle_input(&mut state)?;

        if matches!(action, AppAction::Quit) {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn draw<T: Backend>(terminal: &mut Terminal<T>, state: &mut ExplorerState) -> Result<()>
where
    T::Error: Send + Sync + 'static,
{
    terminal.draw(|frame: &mut Frame<'_>| {
        state.height = frame.area().height.saturating_sub(5) as usize;

        let v_chunks = Layout::default()
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .spacing(Spacing::Overlap(1))
            .split(frame.area());

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                if state.info_open {
                    Constraint::Percentage(50)
                } else {
                    Constraint::Length(0)
                },
            ])
            .spacing(Spacing::Overlap(1))
            .split(v_chunks[0]);

        // We need to virtualize this tree, otherwise it's too big
        let tree_slice =
            (state.scroll_offset)..(state.scroll_offset + state.height).min(state.flat_list.len());

        let items: Vec<ListItem> = state.flat_list[tree_slice]
            .iter()
            .map(|(node, depth)| {
                let prefix = "  ".repeat(*depth);
                let expand_marker = match state.expanded.contains(&node.id) {
                    _ if node.children.is_empty() => "  ",
                    true => "▼ ",
                    false => "▶ ",
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::raw(expand_marker),
                    Span::styled(
                        format!("{:>7}  ", format_bytes(node.retained_size)),
                        Style::default().fg(Color::Yellow),
                    ),
                    if matches!(node.id, UiTreeId::Heap(_)) {
                        Span::raw(&node.label)
                    } else {
                        Span::styled(&node.label, Style::default().fg(Color::Green))
                    },
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::bordered()
                    .merge_borders(MergeStrategy::Exact)
                    .title("Dominator Tree"),
            );

        frame.render_stateful_widget(
            list,
            chunks[0],
            &mut ratatui::widgets::ListState::default()
                .with_selected(Some(state.selected - state.scroll_offset)),
        );

        if state.info_open {
            frame.render_widget(
                Paragraph::new("info").block(
                    Block::bordered()
                        .title("Inspector")
                        .merge_borders(MergeStrategy::Exact)
                        .padding(Padding::horizontal(2)),
                ),
                chunks[1],
            );
        }

        frame.render_widget(
            Paragraph::new(
                "←/↓/↑/→ h/j/k/l: Navigate | Enter/Space: Toggle | i: Inspector | q: Quit",
            )
            .block(
                Block::bordered()
                    .merge_borders(MergeStrategy::Exact)
                    .padding(Padding::horizontal(2)),
            ),
            v_chunks[1],
        );
    })?;
    Ok(())
}

enum AppAction {
    Quit,
    Continue,
}

fn handle_input(state: &mut ExplorerState) -> Result<AppAction> {
    if event::poll(std::time::Duration::from_millis(1000))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(AppAction::Quit),
                    KeyCode::Char('g') => state.move_selection(isize::MIN),
                    KeyCode::Down | KeyCode::Char('j') => state.move_selection(1),
                    KeyCode::PageDown | KeyCode::Char('J') => {
                        state.move_selection(state.height as isize)
                    }
                    KeyCode::Up | KeyCode::Char('k') => state.move_selection(-1),
                    KeyCode::PageUp | KeyCode::Char('K') => {
                        state.move_selection(-(state.height as isize))
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        state.expand_selected();
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        state.toggle_selected();
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        state.collapse_selected();
                    }
                    KeyCode::Char('i') => {
                        state.info_open = !state.info_open;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(AppAction::Continue)
}

/// Build a UI tree from the given graph and node
fn build_ui_tree(node_id: NodeId, tree: &DominatorTree, graph: &V8HeapGraph) -> UiTreeNode {
    let node = graph.node(node_id);
    let retained_size = tree.retained_size(node_id);
    let label = minimal_node_repr(node.id, graph);

    let mut children = if let Some(child_ids) = tree.children.get(&node_id) {
        child_ids
            .iter()
            .filter(|&&n| {
                !matches!(
                    graph.node(n).typ(),
                    NodeType::Hidden
                        | NodeType::ObjectShape
                        | NodeType::ConcatString
                        | NodeType::SlicedString
                        | NodeType::Code
                        | NodeType::Array
                )
            })
            .map(|&child| build_ui_tree(child, tree, graph))
            .collect()
    } else {
        vec![]
    };

    children.sort_by_key(|n| -(n.retained_size as isize));

    UiTreeNode {
        id: UiTreeId::Heap(node_id),
        label,
        retained_size,
        children,
    }
}

/// Find and insert groups into this tree
///
/// We group nodes if they occur at the same level in the dominator tree and have the same minimal rendering (label)
fn find_groups_in_ui_tree(tree: &mut UiTreeNode) {
    let mut ctr = 0;
    find_groups_in_ui_tree_rec(tree, &mut ctr);
}

fn find_groups_in_ui_tree_rec(tree: &mut UiTreeNode, group_counter: &mut usize) {
    // Only for heap nodes
    if matches!(tree.id, UiTreeId::Heap(_)) {
        let mut labels: HashMap<String, Vec<usize>> = Default::default();
        for (i, child) in tree.children.iter().enumerate() {
            labels.entry(child.label.clone()).or_default().push(i);
        }

        if labels.iter().any(|(_, ixes)| ixes.len() > 1) {
            let mut old_children = std::mem::take(&mut tree.children);

            // We have duplicates. The easiest way to deal with this is to rebuild the entire "children" list for this tree node.
            tree.children = labels
                .into_iter()
                .map(|(_, indexes)| {
                    if indexes.len() == 1 {
                        std::mem::take(&mut old_children[indexes[0]])
                    } else {
                        let retained_size =
                            indexes.iter().map(|&i| old_children[i].retained_size).sum();
                        let children = indexes
                            .iter()
                            .map(|&i| std::mem::take(&mut old_children[i]))
                            .collect_vec();

                        let ret = UiTreeNode {
                            id: UiTreeId::Group(*group_counter),
                            label: format!(
                                "<Group> {} instances of {}",
                                indexes.len(),
                                children[0].label
                            ),
                            retained_size,
                            children,
                        };
                        *group_counter += 1;
                        ret
                    }
                })
                .collect_vec();

            tree.children.sort_by_key(|n| -(n.retained_size as isize));
        }
    }

    for child in &mut tree.children {
        find_groups_in_ui_tree_rec(child, group_counter);
    }
}

/// Flattens the tree out to a list of renderable records, based on the expanded nodes.
fn flatten_tree<'a>(
    node: &'a UiTreeNode,
    expanded: &HashSet<UiTreeId>,
) -> Vec<(&'a UiTreeNode, usize)> {
    let mut result = vec![];
    flatten_recursive(node, expanded, &mut result, 0);
    result
}

fn flatten_recursive<'a>(
    node: &'a UiTreeNode,
    expanded: &HashSet<UiTreeId>,
    result: &mut Vec<(&'a UiTreeNode, usize)>,
    depth: usize,
) {
    result.push((node, depth));

    if expanded.contains(&node.id) {
        for child in &node.children {
            flatten_recursive(child, expanded, result, depth + 1);
        }
    }
}

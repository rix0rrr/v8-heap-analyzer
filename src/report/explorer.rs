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
    widgets::{Block, List, ListItem, Padding, Paragraph, Wrap},
};
use std::io;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use crate::{
    analysis::{all_paths::RootPaths, dominator_tree::DominatorTree},
    graph::v8_heap_graph::{NodeType, V8HeapGraph},
    report::{detailed_node_repr, format_retention_paths, minimal_node_repr},
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

struct FlatUiTreeNode<'a> {
    node: &'a UiTreeNode,
    depth: usize,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Focus {
    Tree,
    Inspector,
}

struct ExplorerState<'a> {
    pub selected: usize,
    pub tree_scroll_offset: usize,
    pub inspector_scroll_offset: u16,
    pub height: usize,
    pub expanded: HashSet<UiTreeId>,
    pub flat_list: Vec<FlatUiTreeNode<'a>>,
    pub root: &'a UiTreeNode,
    pub info_open: bool,
    pub focus: Focus,
}

impl<'a> ExplorerState<'a> {
    pub fn new(root: &'a UiTreeNode) -> Self {
        let mut expanded = HashSet::<UiTreeId>::new();
        expanded.insert(UiTreeId::Heap(0)); // Root starts expanded

        let flat_list = flatten_tree(root, &expanded);

        ExplorerState {
            selected: 0,
            tree_scroll_offset: 0,
            inspector_scroll_offset: 0,
            height: 0,
            expanded,
            flat_list,
            root,
            info_open: false,
            focus: Focus::Tree,
        }
    }

    pub fn set_selection(&mut self, selected: usize) {
        if selected != self.selected {
            self.inspector_scroll_offset = 0;
        }
        self.selected = selected;
        if self.selected >= self.tree_scroll_offset + self.height {
            self.tree_scroll_offset = self.selected - self.height + 1;
        }
        if self.selected < self.tree_scroll_offset {
            self.tree_scroll_offset = self.selected;
        }
    }

    pub fn selected_node(&self) -> &UiTreeNode {
        self.flat_list[self.selected].node
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
        if !self.selected_node().children.is_empty() {
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
        if !self.selected_node().children.is_empty() && !self.expanded.contains(&node_id) {
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
            let current_depth = self.flat_list[self.selected].depth;
            if current_depth > 0 {
                for i in (0..self.selected).rev() {
                    if self.flat_list[i].depth < current_depth {
                        let parent_id = self.flat_list[i].node.id;
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
        self.selected_node().id
    }

    fn update_flat_list(&mut self) {
        self.flat_list = flatten_tree(self.root, &self.expanded);
    }
}

pub fn explore_graph(
    tree: &DominatorTree,
    root_paths: &RootPaths,
    graph: &V8HeapGraph,
) -> Result<()> {
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
        draw(&mut terminal, &mut state, root_paths, graph)?;
        let action = handle_input(&mut state)?;

        if matches!(action, AppAction::Quit) {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn draw<T: Backend>(
    terminal: &mut Terminal<T>,
    state: &mut ExplorerState,
    root_paths: &RootPaths,
    graph: &V8HeapGraph,
) -> Result<()>
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
            (state.tree_scroll_offset)..(state.tree_scroll_offset + state.height).min(state.flat_list.len());

        let items: Vec<ListItem> = state.flat_list[tree_slice]
            .iter()
            .map(|node| {
                let prefix = "  ".repeat(node.depth);
                let expand_marker = match state.expanded.contains(&node.node.id) {
                    _ if node.node.children.is_empty() => "  ",
                    true => "▼ ",
                    false => "▶ ",
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::raw(expand_marker),
                    Span::styled(
                        format!("{:>7}  ", format_bytes(node.node.retained_size)),
                        Style::default().fg(Color::Yellow),
                    ),
                    if matches!(node.node.id, UiTreeId::Heap(_)) {
                        Span::raw(&node.node.label)
                    } else {
                        Span::styled(&node.node.label, Style::default().fg(Color::Green))
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
            &mut {
                let mut x = ratatui::widgets::ListState::default();
                if state.focus == Focus::Tree {
                    x = x.with_selected(Some(state.selected - state.tree_scroll_offset));
                }
                x
            },
        );

        if state.info_open {
            frame.render_widget(
                render_inspector(state.selected_node(), root_paths, graph)
                .scroll((0, state.inspector_scroll_offset))
                .block(
                {
                    let mut x = Block::bordered()
                        .title("Inspector")
                        .merge_borders(MergeStrategy::Exact)
                        .padding(Padding::horizontal(2));
                    if state.focus == Focus::Inspector {
                        x = x.border_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow));
                    }
                    x
                }),
                chunks[1],
            );
        }

        frame.render_widget(
            Paragraph::new(
                "←/↓/↑/→ h/j/k/l: Navigate | Enter/Space: Toggle | i: Inspector | <Tab>: move focus | q: Quit",
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

fn render_inspector<'a>(
    ui_tree_node: &'a UiTreeNode,
    root_paths: &'a RootPaths,
    graph: &'a V8HeapGraph,
) -> Paragraph<'a> {
    match &ui_tree_node.id {
        UiTreeId::Group(_) => Paragraph::new(ui_tree_node.label.clone()),
        UiTreeId::Heap(node_id) => {
            let mut s = detailed_node_repr(*node_id, graph);
            let _ = write!(&mut s, "\n\nPath(s):\n");
            let _ = format_retention_paths(&mut s, *node_id, root_paths, graph);

            Paragraph::new(s).wrap(Wrap::default())
        }
    }
}

enum AppAction {
    Quit,
    Continue,
}

fn handle_input(state: &mut ExplorerState) -> Result<AppAction> {
    if event::poll(std::time::Duration::from_millis(1000))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        if state.focus == Focus::Tree {
            match key.code {
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
                _ => {}
            }
        }

        if state.focus == Focus::Inspector {
            match key.code {
                KeyCode::Char('g') => state.inspector_scroll_offset = 0,
                KeyCode::Down | KeyCode::Char('j') => state.inspector_scroll_offset += 1,
                KeyCode::PageDown | KeyCode::Char('J') => {
                    state.inspector_scroll_offset += state.height as u16;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.inspector_scroll_offset = state.inspector_scroll_offset.saturating_sub(1)
                }
                KeyCode::PageUp | KeyCode::Char('K') => {
                    state.inspector_scroll_offset = state
                        .inspector_scroll_offset
                        .saturating_sub(state.height as u16);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => return Ok(AppAction::Quit),
            KeyCode::Char('i') => {
                state.info_open = !state.info_open;
                if !state.info_open && state.focus == Focus::Inspector {
                    state.focus = Focus::Tree;
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                state.focus = match state.focus {
                    Focus::Tree if state.info_open => Focus::Inspector,
                    Focus::Inspector => Focus::Tree,
                    _ => state.focus,
                }
            }
            _ => {}
        }
    }
    Ok(AppAction::Continue)
}

/// Build a UI tree from the given graph and node
fn build_ui_tree(node_id: NodeId, tree: &DominatorTree, graph: &V8HeapGraph) -> UiTreeNode {
    build_ui_tree_rec(node_id, tree, graph)
}

fn build_ui_tree_rec(node_id: NodeId, tree: &DominatorTree, graph: &V8HeapGraph) -> UiTreeNode {
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
            .map(|&child| build_ui_tree_rec(child, tree, graph))
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
                .into_values()
                .map(|indexes| {
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
fn flatten_tree<'a>(node: &'a UiTreeNode, expanded: &HashSet<UiTreeId>) -> Vec<FlatUiTreeNode<'a>> {
    let mut result = vec![];
    flatten_recursive(node, expanded, &mut result, 0);
    result
}

fn flatten_recursive<'a>(
    node: &'a UiTreeNode,
    expanded: &HashSet<UiTreeId>,
    result: &mut Vec<FlatUiTreeNode<'a>>,
    depth: usize,
) {
    result.push(FlatUiTreeNode { node, depth });

    if expanded.contains(&node.id) {
        for child in &node.children {
            flatten_recursive(child, expanded, result, depth + 1);
        }
    }
}

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};
use std::collections::HashSet;
use std::io;

use crate::{
    analysis::dominator_tree::DominatorTree,
    graph::v8_heap_graph::{NodeType, V8HeapGraph},
    report::minimal_node_repr,
    types::NodeId,
    utils::format_bytes,
};

struct TreeNode {
    id: NodeId,
    label: String,
    retained_size: usize,
    children: Vec<TreeNode>,
}

struct ExplorerState<'a> {
    pub selected: usize,
    pub scroll_offset: usize,
    pub height: usize,
    pub expanded: HashSet<NodeId>,
    pub flat_list: Vec<(&'a TreeNode, usize)>,
    pub root: &'a TreeNode,
}

impl<'a> ExplorerState<'a> {
    pub fn new(root: &'a TreeNode) -> Self {
        let mut expanded = HashSet::<NodeId>::new();
        expanded.insert(0); // Root starts expanded

        let flat_list = flatten_tree(&root, &expanded);

        ExplorerState {
            selected: 0,
            scroll_offset: 0,
            height: 0,
            expanded,
            flat_list,
            root,
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if delta > 0 {
            self.selected = (self.selected + delta as usize).min(self.flat_list.len() - 1)
        } else {
            self.selected = self.selected.saturating_sub((-delta) as usize);
        }
        if self.selected >= self.scroll_offset + self.height {
            self.scroll_offset = self.selected - self.height + 1;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
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
                            self.selected = i;
                        }
                        break;
                    }
                }
            }
        }
    }

    fn selected_id(&self) -> NodeId {
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
    let root = build_tree_node(0, tree, graph, 0);
    let mut state = ExplorerState::new(&root);

    loop {
        terminal.draw(&mut |frame: &mut Frame<'_>| {
            state.height = frame.area().height.saturating_sub(5) as usize;

            let chunks = Layout::default()
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(frame.area());

            let items: Vec<ListItem> = state
                .flat_list
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
                        Span::raw(format!("  {:>7}  ", format_bytes(node.retained_size))),
                        Span::raw(&node.label),
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
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Dominator Tree"),
                );

            frame.render_stateful_widget(
                list,
                chunks[0],
                &mut ratatui::widgets::ListState::default()
                    .with_selected(Some(state.selected))
                    .with_offset(state.scroll_offset),
            );

            let help = Paragraph::new("←/↓/↑/→ h/j/k/l: Navigate | Enter/Space: Toggle | q: Quit")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .padding(Padding::horizontal(2)),
                );
            frame.render_widget(help, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(1000))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
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
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn build_tree_node(
    node_id: NodeId,
    tree: &DominatorTree,
    graph: &V8HeapGraph,
    depth: usize,
) -> TreeNode {
    let node = graph.node(node_id);
    let retained_size = tree.retained_sizes[node_id as usize];
    let label = minimal_node_repr(node.id, graph);

    let children = if let Some(mut child_ids) = tree.children.get(&node_id).cloned() {
        child_ids.sort_by_key(|n| -(tree.retained_sizes[*n as usize] as i64));
        child_ids.retain(|n| {
            !matches!(
                graph.node(*n).typ(),
                NodeType::Hidden
                    | NodeType::ObjectShape
                    | NodeType::ConcatString
                    | NodeType::SlicedString
                    | NodeType::Code
                    | NodeType::Array
            )
        });

        child_ids
            .iter()
            .map(|&child| build_tree_node(child, tree, graph, depth + 1))
            .collect()
    } else {
        vec![]
    };

    TreeNode {
        id: node_id,
        label,
        retained_size,
        children,
    }
}

fn flatten_tree<'a>(node: &'a TreeNode, expanded: &HashSet<NodeId>) -> Vec<(&'a TreeNode, usize)> {
    let mut result = vec![];
    flatten_recursive(node, expanded, &mut result, 0);
    result
}

fn flatten_recursive<'a>(
    node: &'a TreeNode,
    expanded: &HashSet<NodeId>,
    result: &mut Vec<(&'a TreeNode, usize)>,
    depth: usize,
) {
    result.push((node, depth));

    if expanded.contains(&node.id) {
        for child in &node.children {
            flatten_recursive(child, expanded, result, depth + 1);
        }
    }
}

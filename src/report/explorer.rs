use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

pub fn explore_graph(tree: &DominatorTree, graph: &V8HeapGraph) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let root = build_tree_node(0, tree, graph, 0);
    let mut selected = 0;
    let mut offset = 0;
    let mut expanded: HashSet<NodeId> = HashSet::new();
    expanded.insert(0); // Root starts expanded
    let mut flat_list = flatten_tree(&root, &expanded);

    loop {
        let height = terminal.size()?.height.saturating_sub(5) as usize;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(f.area());

            let items: Vec<ListItem> = flat_list
                .iter()
                .skip(offset)
                .take(height)
                .enumerate()
                .map(|(i, (node, depth))| {
                    let actual_index = offset + i;
                    let prefix = "  ".repeat(*depth);
                    let expand_marker = if !node.children.is_empty() {
                        if expanded.contains(&node.id) {
                            "▼ "
                        } else {
                            "▶ "
                        }
                    } else {
                        "  "
                    };

                    let style = Style::default();
                    let label_style = Style::default();

                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, style),
                        Span::styled(expand_marker, style),
                        Span::styled(&node.label, label_style),
                        Span::styled(format!("  ({})", format_bytes(node.retained_size)), style),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Dominator Tree"),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(
                list,
                chunks[0],
                &mut ratatui::widgets::ListState::default().with_selected(Some(selected)),
            );

            let help = Paragraph::new(
                "↑/↓: Navigate | →: Expand | ←: Collapse | Enter/Space: Toggle | q: Quit",
            )
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down => {
                            if selected < flat_list.len() - 1 {
                                selected += 1;
                                if selected >= offset + height {
                                    offset = selected - height + 1;
                                }
                            }
                        }
                        KeyCode::Up => {
                            if selected > 0 {
                                selected -= 1;
                                if selected < offset {
                                    offset = selected;
                                }
                            }
                        }
                        KeyCode::Right => {
                            let node_id = flat_list[selected].0.id;
                            if !flat_list[selected].0.children.is_empty()
                                && !expanded.contains(&node_id)
                            {
                                expanded.insert(node_id);
                                flat_list = flatten_tree(&root, &expanded);
                            }
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            let node_id = flat_list[selected].0.id;
                            if !flat_list[selected].0.children.is_empty() {
                                if expanded.contains(&node_id) {
                                    expanded.remove(&node_id);
                                } else {
                                    expanded.insert(node_id);
                                }
                                flat_list = flatten_tree(&root, &expanded);
                            }
                        }
                        KeyCode::Left => {
                            let node_id = flat_list[selected].0.id;
                            if expanded.contains(&node_id) {
                                expanded.remove(&node_id);
                                flat_list = flatten_tree(&root, &expanded);
                            } else {
                                // Find parent and collapse it
                                let current_depth = flat_list[selected].1;
                                if current_depth > 0 {
                                    for i in (0..selected).rev() {
                                        if flat_list[i].1 < current_depth {
                                            let parent_id = flat_list[i].0.id;
                                            if expanded.contains(&parent_id) {
                                                expanded.remove(&parent_id);
                                                flat_list = flatten_tree(&root, &expanded);
                                                selected = i;
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
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
    let label = format!(
        "[{}] {}",
        node.stable_id(),
        minimal_node_repr(node.id, graph)
    );

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

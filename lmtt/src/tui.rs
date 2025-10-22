use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lmtt_core::{Config, ThemeMode};
use lmtt_modules::ModuleRegistry;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use schema_tui::tui::{TextInput, Toggle, NumberInput, Dropdown, Widget, WidgetResult};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConfigTab {
    General,
    Notifications,
    Performance,
    Modules,
    Cache,
    Logging,
}

impl ConfigTab {
    fn all() -> Vec<Self> {
        vec![
            Self::General,
            Self::Notifications,
            Self::Performance,
            Self::Modules,
            Self::Cache,
            Self::Logging,
        ]
    }
    
    fn name(&self) -> &str {
        match self {
            Self::General => "General",
            Self::Notifications => "Notifications",
            Self::Performance => "Performance",
            Self::Modules => "Modules",
            Self::Cache => "Cache",
            Self::Logging => "Logging",
        }
    }
}

enum ModuleStatus {
    Enabled,
    Disabled,
    NotInstalled,
}

struct ModuleInfo {
    name: String,
    status: ModuleStatus,
    #[allow(dead_code)]
    description: String,
}

pub struct TuiApp {
    config: Config,
    modules: Vec<ModuleInfo>,
    current_tab: ConfigTab,
    list_state: ListState,
    should_quit: bool,
    message: Option<String>,
    edit_mode: bool,
    edit_buffer: String,
    edit_cursor_pos: usize,
    dropdown_mode: bool,
    dropdown_options: Vec<String>,
    dropdown_state: ListState,
}

impl TuiApp {
    pub fn new(config: Config) -> Self {
        let registry = ModuleRegistry::new();
        let mut modules = Vec::new();
        
        for module in &registry.modules {
            let name = module.name().to_string();
            let is_enabled = config.is_module_enabled(&name);
            let is_installed = module.is_installed();
            
            let status = if !is_installed {
                ModuleStatus::NotInstalled
            } else if is_enabled {
                ModuleStatus::Enabled
            } else {
                ModuleStatus::Disabled
            };
            
            modules.push(ModuleInfo {
                name: name.clone(),
                status,
                description: format!("Binary: {}", module.binary_name()),
            });
        }
        
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        Self {
            config,
            modules,
            current_tab: ConfigTab::General,
            list_state,
            should_quit: false,
            message: None,
            edit_mode: false,
            edit_buffer: String::new(),
            edit_cursor_pos: 0,
            dropdown_mode: false,
            dropdown_options: Vec::new(),
            dropdown_state: ListState::default(),
        }
    }
    
    pub fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        let result = self.run_loop(&mut terminal);
        
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        
        result
    }
    
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;
            
            if self.should_quit {
                break;
            }
            
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.dropdown_mode {
            match key.code {
                KeyCode::Enter => {
                    self.select_dropdown_option()?;
                    self.dropdown_mode = false;
                }
                KeyCode::Esc => {
                    self.dropdown_mode = false;
                    self.message = Some("Selection cancelled".to_string());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = match self.dropdown_state.selected() {
                        Some(i) => (i + 1) % self.dropdown_options.len(),
                        None => 0,
                    };
                    self.dropdown_state.select(Some(i));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = match self.dropdown_state.selected() {
                        Some(i) if i == 0 => self.dropdown_options.len() - 1,
                        Some(i) => i - 1,
                        None => 0,
                    };
                    self.dropdown_state.select(Some(i));
                }
                _ => {}
            }
        } else if self.edit_mode {
            match key.code {
                KeyCode::Enter => {
                    self.save_edit()?;
                    self.edit_mode = false;
                    self.edit_buffer.clear();
                    self.edit_cursor_pos = 0;
                }
                KeyCode::Esc => {
                    self.edit_mode = false;
                    self.edit_buffer.clear();
                    self.edit_cursor_pos = 0;
                    self.message = Some("Edit cancelled".to_string());
                }
                KeyCode::Backspace => {
                    if self.edit_cursor_pos > 0 {
                        self.edit_buffer.remove(self.edit_cursor_pos - 1);
                        self.edit_cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    if self.edit_cursor_pos < self.edit_buffer.len() {
                        self.edit_buffer.remove(self.edit_cursor_pos);
                    }
                }
                KeyCode::Left => {
                    if self.edit_cursor_pos > 0 {
                        self.edit_cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.edit_cursor_pos < self.edit_buffer.len() {
                        self.edit_cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    self.edit_cursor_pos = 0;
                }
                KeyCode::End => {
                    self.edit_cursor_pos = self.edit_buffer.len();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.insert(self.edit_cursor_pos, c);
                    self.edit_cursor_pos += 1;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.should_quit = true;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    self.next_tab();
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                    self.previous_tab();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.next_item();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.previous_item();
                }
                KeyCode::Enter => {
                    self.handle_enter()?;
                }
                KeyCode::Char(' ') => {
                    self.handle_space()?;
                }
                KeyCode::Char('e') => {
                    self.handle_e_key()?;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn next_tab(&mut self) {
        let tabs = ConfigTab::all();
        let current_idx = tabs.iter().position(|t| t == &self.current_tab).unwrap();
        self.current_tab = tabs[(current_idx + 1) % tabs.len()];
        self.list_state.select(Some(0));
        self.message = None;
    }
    
    fn previous_tab(&mut self) {
        let tabs = ConfigTab::all();
        let current_idx = tabs.iter().position(|t| t == &self.current_tab).unwrap();
        self.current_tab = if current_idx == 0 {
            *tabs.last().unwrap()
        } else {
            tabs[current_idx - 1]
        };
        self.list_state.select(Some(0));
        self.message = None;
    }
    
    fn next_item(&mut self) {
        let item_count = self.get_item_count();
        if item_count == 0 {
            return;
        }
        
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % item_count,
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn previous_item(&mut self) {
        let item_count = self.get_item_count();
        if item_count == 0 {
            return;
        }
        
        let i = match self.list_state.selected() {
            Some(i) if i == 0 => item_count - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn get_item_count(&self) -> usize {
        match self.current_tab {
            ConfigTab::General => 6,
            ConfigTab::Notifications => 3,
            ConfigTab::Performance => 2,
            ConfigTab::Modules => self.modules.len(),
            ConfigTab::Cache => 2,
            ConfigTab::Logging => 3,
        }
    }
    
    fn handle_enter(&mut self) -> anyhow::Result<()> {
        let selected = match self.list_state.selected() {
            Some(s) => s,
            None => return Ok(()),
        };
        
        match self.current_tab {
            ConfigTab::General => {
                match selected {
                    0 => { // wallpaper
                        self.edit_buffer = self.config.general.wallpaper.clone();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: wallpaper (Enter to save, Esc to cancel)".to_string());
                    }
                    1 => { // default_mode - dropdown
                        self.dropdown_options = vec!["Light".to_string(), "Dark".to_string()];
                        self.dropdown_state.select(Some(match self.config.general.default_mode {
                            ThemeMode::Light => 0,
                            ThemeMode::Dark => 1,
                        }));
                        self.dropdown_mode = true;
                        self.message = Some("Select mode (↑↓ to navigate, Enter to select)".to_string());
                    }
                    2 => { // scheme_type - dropdown
                        self.dropdown_options = vec![
                            "scheme-content".to_string(),
                            "scheme-expressive".to_string(),
                            "scheme-fidelity".to_string(),
                            "scheme-fruit-salad".to_string(),
                            "scheme-monochrome".to_string(),
                            "scheme-neutral".to_string(),
                            "scheme-rainbow".to_string(),
                            "scheme-tonal-spot".to_string(),
                            "scheme-vibrant".to_string(),
                        ];
                        let current_idx = self.dropdown_options.iter()
                            .position(|s| s == &self.config.general.scheme_type)
                            .unwrap_or(1); // default to expressive
                        self.dropdown_state.select(Some(current_idx));
                        self.dropdown_mode = true;
                        self.message = Some("Select scheme (↑↓ to navigate, Enter to select)".to_string());
                    }
                    3 => { // use_matugen
                        self.config.general.use_matugen = !self.config.general.use_matugen;
                        self.message = Some(format!("Use matugen: {}", self.config.general.use_matugen));
                        self.config.save()?;
                    }
                    4 => { // default_light_colors
                        self.edit_buffer = self.config.general.default_light_colors.clone();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: default_light_colors (Enter to save, Esc to cancel)".to_string());
                    }
                    5 => { // default_dark_colors
                        self.edit_buffer = self.config.general.default_dark_colors.clone();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: default_dark_colors (Enter to save, Esc to cancel)".to_string());
                    }
                    _ => {}
                }
            }
            ConfigTab::Notifications => {
                match selected {
                    0 => { // enabled
                        self.config.notifications.enabled = !self.config.notifications.enabled;
                        self.message = Some(format!("Notifications: {}", self.config.notifications.enabled));
                        self.config.save()?;
                    }
                    1 => { // timeout
                        self.edit_buffer = self.config.notifications.timeout.to_string();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: timeout (Enter to save, Esc to cancel)".to_string());
                    }
                    2 => { // show_module_progress
                        self.config.notifications.show_module_progress = !self.config.notifications.show_module_progress;
                        self.message = Some(format!("Show progress: {}", self.config.notifications.show_module_progress));
                        self.config.save()?;
                    }
                    _ => {}
                }
            }
            ConfigTab::Performance => {
                match selected {
                    0 => { // timeout
                        self.edit_buffer = self.config.performance.timeout.to_string();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: timeout (Enter to save, Esc to cancel)".to_string());
                    }
                    1 => { // slow_module_threshold
                        self.edit_buffer = self.config.performance.slow_module_threshold.to_string();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: slow_module_threshold (Enter to save, Esc to cancel)".to_string());
                    }
                    _ => {}
                }
            }
            ConfigTab::Modules => {
                self.handle_space()?;
            }
            ConfigTab::Cache => {
                match selected {
                    0 => { // enabled
                        self.config.cache.enabled = !self.config.cache.enabled;
                        self.message = Some(format!("Cache: {}", self.config.cache.enabled));
                        self.config.save()?;
                    }
                    1 => { // dir
                        self.edit_buffer = self.config.cache.dir.clone();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: dir (Enter to save, Esc to cancel)".to_string());
                    }
                    _ => {}
                }
            }
            ConfigTab::Logging => {
                match selected {
                    0 => { // level - dropdown
                        self.dropdown_options = vec![
                            "trace".to_string(),
                            "debug".to_string(),
                            "info".to_string(),
                            "warn".to_string(),
                            "error".to_string(),
                        ];
                        let current_idx = self.dropdown_options.iter()
                            .position(|s| s == &self.config.logging.level)
                            .unwrap_or(2); // default to info
                        self.dropdown_state.select(Some(current_idx));
                        self.dropdown_mode = true;
                        self.message = Some("Select log level (↑↓ to navigate, Enter to select)".to_string());
                    }
                    1 => { // log_file
                        self.edit_buffer = self.config.logging.log_file.clone();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: log_file (Enter to save, Esc to cancel)".to_string());
                    }
                    2 => { // max_log_size
                        self.edit_buffer = self.config.logging.max_log_size.to_string();
                        self.edit_cursor_pos = self.edit_buffer.len();
                        self.edit_mode = true;
                        self.message = Some("Editing: max_log_size (Enter to save, Esc to cancel)".to_string());
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    fn select_dropdown_option(&mut self) -> anyhow::Result<()> {
        let selected_field = match self.list_state.selected() {
            Some(s) => s,
            None => return Ok(()),
        };
        
        let selected_option = match self.dropdown_state.selected() {
            Some(s) => s,
            None => return Ok(()),
        };
        
        let option_value = self.dropdown_options.get(selected_option)
            .ok_or(anyhow::anyhow!("Invalid dropdown selection"))?
            .clone();
        
        match self.current_tab {
            ConfigTab::General => {
                if selected_field == 1 { // default_mode
                    self.config.general.default_mode = match option_value.as_str() {
                        "Light" => ThemeMode::Light,
                        "Dark" => ThemeMode::Dark,
                        _ => ThemeMode::Dark,
                    };
                    self.message = Some(format!("Set default_mode to {}", option_value));
                } else if selected_field == 2 { // scheme_type
                    self.config.general.scheme_type = option_value.clone();
                    self.message = Some(format!("Set scheme_type to {}", option_value));
                }
            }
            ConfigTab::Logging => {
                if selected_field == 0 { // level
                    self.config.logging.level = option_value.clone();
                    self.message = Some(format!("Set log level to {}", option_value));
                }
            }
            _ => {}
        }
        
        self.config.save()?;
        Ok(())
    }
    
    fn save_edit(&mut self) -> anyhow::Result<()> {
        let selected = match self.list_state.selected() {
            Some(s) => s,
            None => return Ok(()),
        };
        
        match self.current_tab {
            ConfigTab::General => {
                match selected {
                    0 => {
                        self.config.general.wallpaper = self.edit_buffer.clone();
                        self.message = Some("Updated wallpaper".to_string());
                    }
                    2 => {
                        self.config.general.scheme_type = self.edit_buffer.clone();
                        self.message = Some("Updated scheme_type".to_string());
                    }
                    4 => {
                        self.config.general.default_light_colors = self.edit_buffer.clone();
                        self.message = Some("Updated default_light_colors".to_string());
                    }
                    5 => {
                        self.config.general.default_dark_colors = self.edit_buffer.clone();
                        self.message = Some("Updated default_dark_colors".to_string());
                    }
                    _ => {}
                }
            }
            ConfigTab::Notifications => {
                if selected == 1 {
                    if let Ok(val) = self.edit_buffer.parse::<i32>() {
                        self.config.notifications.timeout = val;
                        self.message = Some("Updated timeout".to_string());
                    } else {
                        self.message = Some("Invalid number".to_string());
                        return Ok(());
                    }
                }
            }
            ConfigTab::Performance => {
                match selected {
                    0 => {
                        if let Ok(val) = self.edit_buffer.parse::<u64>() {
                            self.config.performance.timeout = val;
                            self.message = Some("Updated timeout".to_string());
                        } else {
                            self.message = Some("Invalid number".to_string());
                            return Ok(());
                        }
                    }
                    1 => {
                        if let Ok(val) = self.edit_buffer.parse::<u64>() {
                            self.config.performance.slow_module_threshold = val;
                            self.message = Some("Updated slow_module_threshold".to_string());
                        } else {
                            self.message = Some("Invalid number".to_string());
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            ConfigTab::Cache => {
                if selected == 1 {
                    self.config.cache.dir = self.edit_buffer.clone();
                    self.message = Some("Updated dir".to_string());
                }
            }
            ConfigTab::Logging => {
                match selected {
                    0 => {
                        self.config.logging.level = self.edit_buffer.clone();
                        self.message = Some("Updated level".to_string());
                    }
                    1 => {
                        self.config.logging.log_file = self.edit_buffer.clone();
                        self.message = Some("Updated log_file".to_string());
                    }
                    2 => {
                        if let Ok(val) = self.edit_buffer.parse::<u64>() {
                            self.config.logging.max_log_size = val;
                            self.message = Some("Updated max_log_size".to_string());
                        } else {
                            self.message = Some("Invalid number".to_string());
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        self.config.save()?;
        Ok(())
    }
    
    fn handle_space(&mut self) -> anyhow::Result<()> {
        let selected = match self.list_state.selected() {
            Some(s) => s,
            None => return Ok(()),
        };
        
        match self.current_tab {
            ConfigTab::General => {
                match selected {
                    1 => { // default_mode
                        self.config.general.default_mode = match self.config.general.default_mode {
                            ThemeMode::Light => ThemeMode::Dark,
                            ThemeMode::Dark => ThemeMode::Light,
                        };
                        self.message = Some("Default mode toggled".to_string());
                        self.config.save()?;
                    }
                    3 => { // use_matugen
                        self.config.general.use_matugen = !self.config.general.use_matugen;
                        self.message = Some(format!("Use matugen: {}", self.config.general.use_matugen));
                        self.config.save()?;
                    }
                    _ => {
                        self.message = Some("Press 'e' to edit in $EDITOR".to_string());
                    }
                }
            }
            ConfigTab::Notifications => {
                match selected {
                    0 => { // enabled
                        self.config.notifications.enabled = !self.config.notifications.enabled;
                        self.message = Some(format!("Notifications: {}", self.config.notifications.enabled));
                        self.config.save()?;
                    }
                    2 => { // show_module_progress
                        self.config.notifications.show_module_progress = !self.config.notifications.show_module_progress;
                        self.message = Some(format!("Show progress: {}", self.config.notifications.show_module_progress));
                        self.config.save()?;
                    }
                    _ => {
                        self.message = Some("Press 'e' to edit in $EDITOR".to_string());
                    }
                }
            }
            ConfigTab::Performance => {
                self.message = Some("Press 'e' to edit in $EDITOR".to_string());
            }
            ConfigTab::Modules => {
                if let Some(module_info) = self.modules.get_mut(selected) {
                    match module_info.status {
                        ModuleStatus::NotInstalled => {
                            self.message = Some("Cannot toggle: module not installed".to_string());
                        }
                        ModuleStatus::Enabled => {
                            self.config.modules.modules
                                .entry(module_info.name.clone())
                                .or_insert(lmtt_core::config::ModuleSetting {
                                    enabled: false,
                                    restart: false,
                                    command: None,
                                })
                                .enabled = false;
                            module_info.status = ModuleStatus::Disabled;
                            self.message = Some(format!("Disabled {}", module_info.name));
                            self.config.save()?;
                        }
                        ModuleStatus::Disabled => {
                            self.config.modules.modules
                                .entry(module_info.name.clone())
                                .or_insert(lmtt_core::config::ModuleSetting {
                                    enabled: true,
                                    restart: false,
                                    command: None,
                                })
                                .enabled = true;
                            module_info.status = ModuleStatus::Enabled;
                            self.message = Some(format!("Enabled {}", module_info.name));
                            self.config.save()?;
                        }
                    }
                }
            }
            ConfigTab::Cache => {
                if selected == 0 {
                    self.config.cache.enabled = !self.config.cache.enabled;
                    self.message = Some(format!("Cache: {}", self.config.cache.enabled));
                    self.config.save()?;
                } else {
                    self.message = Some("Press 'e' to edit in $EDITOR".to_string());
                }
            }
            ConfigTab::Logging => {
                self.message = Some("Press 'e' to edit in $EDITOR".to_string());
            }
        }
        
        Ok(())
    }
    
    fn handle_e_key(&mut self) -> anyhow::Result<()> {
        let selected = match self.list_state.selected() {
            Some(s) => s,
            None => {
                if self.current_tab == ConfigTab::Modules {
                    self.open_modules_dir()?;
                } else {
                    self.open_in_editor()?;
                }
                return Ok(());
            }
        };
        
        match self.current_tab {
            ConfigTab::General => {
                match selected {
                    0 => self.open_file_in_editor(&self.config.general.wallpaper.clone())?,
                    4 => self.open_file_in_editor(&self.config.general.default_light_colors.clone())?,
                    5 => self.open_file_in_editor(&self.config.general.default_dark_colors.clone())?,
                    _ => self.open_in_editor()?,
                }
            }
            ConfigTab::Cache => {
                if selected == 1 {
                    self.open_file_in_editor(&self.config.cache.dir.clone())?;
                } else {
                    self.open_in_editor()?;
                }
            }
            ConfigTab::Logging => {
                if selected == 1 {
                    self.open_file_in_editor(&self.config.logging.log_file.clone())?;
                } else {
                    self.open_in_editor()?;
                }
            }
            ConfigTab::Modules => {
                self.open_modules_dir()?;
            }
            _ => {
                self.open_in_editor()?;
            }
        }
        
        Ok(())
    }
    
    fn open_modules_dir(&mut self) -> anyhow::Result<()> {
        let modules_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("lmtt")
            .join("modules");
        
        if !modules_dir.exists() {
            std::fs::create_dir_all(&modules_dir)?;
        }
        
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(editor)
            .arg(&modules_dir)
            .status()?;
        
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        
        self.message = Some("Closed modules directory".to_string());
        Ok(())
    }
    
    fn open_file_in_editor(&mut self, path: &str) -> anyhow::Result<()> {
        let expanded_path = if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                path.replacen("~", &home.display().to_string(), 1)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };
        
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(editor)
            .arg(&expanded_path)
            .status()?;
        
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        
        self.message = Some(format!("Closed {}", path));
        Ok(())
    }
    
    fn open_in_editor(&mut self) -> anyhow::Result<()> {
        let config_path = Config::config_path()?;
        
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(editor)
            .arg(&config_path)
            .status()?;
        
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        
        self.config = Config::load()?;
        self.message = Some("Config reloaded from file".to_string());
        
        Ok(())
    }
    
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(5),
            ])
            .split(f.area());
        
        self.render_header(f, chunks[0]);
        self.render_tabs(f, chunks[1]);
        self.render_content(f, chunks[2]);
        self.render_footer(f, chunks[3]);
    }
    
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("LMTT Configuration", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("Config: "),
                Span::styled(
                    Config::config_path().ok().map(|p| p.display().to_string()).unwrap_or_default(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(header, area);
    }
    
    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let tabs = ConfigTab::all();
        let titles: Vec<&str> = tabs.iter().map(|t| t.name()).collect();
        let selected_idx = tabs.iter().position(|t| t == &self.current_tab).unwrap();
        
        let tabs_widget = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Sections (Tab/←→/h/l to switch)"))
            .select(selected_idx)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        
        f.render_widget(tabs_widget, area);
    }
    
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            ConfigTab::General => self.render_general(f, area),
            ConfigTab::Notifications => self.render_notifications(f, area),
            ConfigTab::Performance => self.render_performance(f, area),
            ConfigTab::Modules => self.render_modules(f, area),
            ConfigTab::Cache => self.render_cache(f, area),
            ConfigTab::Logging => self.render_logging(f, area),
        }
        
        // Render dropdown overlay if active
        if self.dropdown_mode {
            self.render_dropdown(f, area);
        }
    }
    
    fn render_dropdown(&mut self, f: &mut Frame, area: Rect) {
        let dropdown_height = (self.dropdown_options.len() + 2).min(10) as u16;
        let dropdown_width = self.dropdown_options.iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(20)
            .max(20) as u16 + 4;
        
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(dropdown_width)) / 2,
            y: area.y + (area.height.saturating_sub(dropdown_height)) / 2,
            width: dropdown_width.min(area.width),
            height: dropdown_height.min(area.height),
        };
        
        let items: Vec<ListItem> = self.dropdown_options
            .iter()
            .map(|opt| ListItem::new(Line::from(opt.as_str())))
            .collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Option (↑↓ to navigate, Enter to select, Esc to cancel)")
                    .style(Style::default().bg(Color::Black))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("» ");
        
        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.dropdown_state);
    }
    
    fn render_general(&mut self, f: &mut Frame, area: Rect) {
        let selected = self.list_state.selected().unwrap_or(0);
        
        let wallpaper_val = if self.edit_mode && selected == 0 {
            self.get_edit_display()
        } else {
            self.config.general.wallpaper.clone()
        };
        
        let scheme_type_val = if self.edit_mode && selected == 2 {
            self.get_edit_display()
        } else {
            self.config.general.scheme_type.clone()
        };
        
        let light_colors_val = if self.edit_mode && selected == 4 {
            self.get_edit_display()
        } else {
            self.config.general.default_light_colors.clone()
        };
        
        let dark_colors_val = if self.edit_mode && selected == 5 {
            self.get_edit_display()
        } else {
            self.config.general.default_dark_colors.clone()
        };
        
        let items = vec![
            format!("wallpaper                = {}", wallpaper_val),
            format!("default_mode             = {:?}", self.config.general.default_mode),
            format!("scheme_type              = {}", scheme_type_val),
            format!("use_matugen              = {}", self.config.general.use_matugen),
            format!("default_light_colors     = {}", light_colors_val),
            format!("default_dark_colors      = {}", dark_colors_val),
        ];
        
        self.render_list(f, area, "General", &items);
    }
    
    fn get_edit_display(&self) -> String {
        let mut display = self.edit_buffer.clone();
        if self.edit_cursor_pos <= display.len() {
            display.insert(self.edit_cursor_pos, '█');
        }
        display
    }
    
    fn render_notifications(&mut self, f: &mut Frame, area: Rect) {
        let selected = self.list_state.selected().unwrap_or(0);
        let timeout_str = self.config.notifications.timeout.to_string();
        let timeout_val = if self.edit_mode && selected == 1 {
            self.get_edit_display()
        } else {
            timeout_str
        };
        
        let items = vec![
            format!("enabled                  = {}", self.config.notifications.enabled),
            format!("timeout                  = {}", timeout_val),
            format!("show_module_progress     = {}", self.config.notifications.show_module_progress),
        ];
        
        self.render_list(f, area, "Notifications", &items);
    }
    
    fn render_performance(&mut self, f: &mut Frame, area: Rect) {
        let selected = self.list_state.selected().unwrap_or(0);
        let timeout_str = self.config.performance.timeout.to_string();
        let threshold_str = self.config.performance.slow_module_threshold.to_string();
        
        let timeout_val = if self.edit_mode && selected == 0 {
            self.get_edit_display()
        } else {
            timeout_str
        };
        
        let threshold_val = if self.edit_mode && selected == 1 {
            self.get_edit_display()
        } else {
            threshold_str
        };
        
        let items = vec![
            format!("timeout                  = {}", timeout_val),
            format!("slow_module_threshold    = {}", threshold_val),
        ];
        
        self.render_list(f, area, "Performance", &items);
    }
    
    fn render_modules(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .modules
            .iter()
            .map(|module_info| {
                let (status_char, status_style) = match module_info.status {
                    ModuleStatus::Enabled => ("✓", Style::default().fg(Color::Green)),
                    ModuleStatus::Disabled => ("✗", Style::default().fg(Color::Red)),
                    ModuleStatus::NotInstalled => ("⊘", Style::default().fg(Color::DarkGray)),
                };
                
                let status_text = match module_info.status {
                    ModuleStatus::Enabled => "enabled",
                    ModuleStatus::Disabled => "disabled",
                    ModuleStatus::NotInstalled => "not installed",
                };
                
                let content = Line::from(vec![
                    Span::styled(format!(" {} ", status_char), status_style),
                    Span::styled(
                        format!("{:<20}", module_info.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(status_text, status_style),
                ]);
                
                ListItem::new(content)
            })
            .collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("[modules] - Space/Enter to toggle"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("» ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    fn render_cache(&mut self, f: &mut Frame, area: Rect) {
        let selected = self.list_state.selected().unwrap_or(0);
        let dir_val = if self.edit_mode && selected == 1 {
            self.get_edit_display()
        } else {
            self.config.cache.dir.clone()
        };
        
        let items = vec![
            format!("enabled                  = {}", self.config.cache.enabled),
            format!("dir                      = {}", dir_val),
        ];
        
        self.render_list(f, area, "Cache", &items);
    }
    
    fn render_logging(&mut self, f: &mut Frame, area: Rect) {
        let selected = self.list_state.selected().unwrap_or(0);
        let max_log_size_str = self.config.logging.max_log_size.to_string();
        
        let log_file_val = if self.edit_mode && selected == 1 {
            self.get_edit_display()
        } else {
            self.config.logging.log_file.clone()
        };
        
        let max_size_val = if self.edit_mode && selected == 2 {
            self.get_edit_display()
        } else {
            max_log_size_str
        };
        
        let items = vec![
            format!("level                    = {}", self.config.logging.level),
            format!("log_file                 = {}", log_file_val),
            format!("max_log_size             = {}", max_size_val),
        ];
        
        self.render_list(f, area, "Logging", &items);
    }
    
    fn render_list(&mut self, f: &mut Frame, area: Rect, title: &str, items: &[String]) {
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|item| ListItem::new(Line::from(item.as_str())))
            .collect();
        
        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("[{}] - Space/Enter to toggle booleans, 'e' to edit in $EDITOR", title)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("» ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    fn get_current_field_info(&self) -> (&'static str, &'static str) {
        let selected = self.list_state.selected().unwrap_or(0);
        
        match self.current_tab {
            ConfigTab::General => {
                let field = match selected {
                    0 => "wallpaper",
                    1 => "default_mode",
                    2 => "scheme_type",
                    3 => "use_matugen",
                    4 => "default_light_colors",
                    5 => "default_dark_colors",
                    _ => "",
                };
                ("general", field)
            }
            ConfigTab::Notifications => {
                let field = match selected {
                    0 => "enabled",
                    1 => "timeout",
                    2 => "show_module_progress",
                    _ => "",
                };
                ("notifications", field)
            }
            ConfigTab::Performance => {
                let field = match selected {
                    0 => "timeout",
                    1 => "slow_module_threshold",
                    _ => "",
                };
                ("performance", field)
            }
            ConfigTab::Cache => {
                let field = match selected {
                    0 => "enabled",
                    1 => "dir",
                    _ => "",
                };
                ("cache", field)
            }
            ConfigTab::Logging => {
                let field = match selected {
                    0 => "level",
                    1 => "log_file",
                    2 => "max_log_size",
                    _ => "",
                };
                ("logging", field)
            }
            ConfigTab::Modules => ("modules", ""),
        }
    }
    
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Tab/←→", Style::default().fg(Color::Cyan)),
                Span::raw(" sections  "),
                Span::styled("↑/k ↓/j", Style::default().fg(Color::Cyan)),
                Span::raw(" navigate  "),
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::raw(" edit  "),
                Span::styled("Space", Style::default().fg(Color::Cyan)),
                Span::raw(" toggle  "),
                Span::styled("e", Style::default().fg(Color::Cyan)),
                Span::raw(" open  "),
                Span::styled("q", Style::default().fg(Color::Cyan)),
                Span::raw(" quit"),
            ]),
        ];
        
        if let Some(msg) = &self.message {
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::raw(msg),
            ]));
        } else {
            lines.push(Line::from("Changes saved automatically"));
        }
        
        // Add field description
        if self.current_tab != ConfigTab::Modules {
            let (section, field) = self.get_current_field_info();
            if !field.is_empty() {
                let description = lmtt_core::Config::get_field_description(section, field);
                lines.push(Line::from(vec![
                    Span::styled("Help: ", Style::default().fg(Color::Green)),
                    Span::styled(description, Style::default().fg(Color::DarkGray)),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("Help: ", Style::default().fg(Color::Green)),
                Span::styled("Space/Enter to toggle module, 'e' to open modules directory", Style::default().fg(Color::DarkGray)),
            ]));
        }
        
        let footer = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Controls"));
        
        f.render_widget(footer, area);
    }
}

pub fn run_tui() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut app = TuiApp::new(config);
    app.run()
}

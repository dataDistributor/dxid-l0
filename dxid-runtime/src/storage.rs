use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{Block, BlockHeader, State, H256};

/// Backup manifest for tracking backup metadata
#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    timestamp: u64,
    backup_name: String,
    files_backed_up: usize,
    blockchain_height: u64,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for all storage
    pub base_dir: PathBuf,
    /// How often to create checkpoints (in blocks)
    pub checkpoint_interval: u64,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
    /// Whether to enable transaction indexing
    pub enable_indexing: bool,
    /// Whether to enable data compression
    pub enable_compression: bool,
    /// Backup interval in seconds
    pub backup_interval_secs: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("./dxid-data"),
            checkpoint_interval: 100, // More frequent checkpoints for better persistence
            max_checkpoints: 50, // Keep more checkpoints
            enable_indexing: true,
            enable_compression: true,
            backup_interval_secs: 1800, // Every 30 minutes for better persistence
        }
    }
}

/// Persistent storage manager
pub struct Storage {
    config: StorageConfig,
    state_file: PathBuf,
    checkpoints_dir: PathBuf,
    index_dir: PathBuf,
    backup_dir: PathBuf,
    last_checkpoint: RwLock<u64>,
    last_backup: RwLock<u64>,
}

impl Storage {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let state_file = config.base_dir.join("state.json");
        let checkpoints_dir = config.base_dir.join("checkpoints");
        let index_dir = config.base_dir.join("index");
        let backup_dir = config.base_dir.join("backups");

        // Create directories
        fs::create_dir_all(&config.base_dir)?;
        fs::create_dir_all(&checkpoints_dir)?;
        fs::create_dir_all(&index_dir)?;
        fs::create_dir_all(&backup_dir)?;

        Ok(Self {
            config,
            state_file,
            checkpoints_dir,
            index_dir,
            backup_dir,
            last_checkpoint: RwLock::new(0),
            last_backup: RwLock::new(0),
        })
    }

    /// Save the current state to disk with multiple backup copies
    pub fn save_state(&self, state: &State) -> Result<()> {
        let temp_file = self.state_file.with_extension("tmp");
        let backup_file = self.state_file.with_extension("backup");
        
        // Write to temporary file first
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_file)
            .context("Failed to create temporary state file")?;
        
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, state)
            .context("Failed to serialize state")?;
        writer.flush().context("Failed to flush state file")?;
        drop(writer);

        // Create backup of current state if it exists
        if self.state_file.exists() {
            fs::copy(&self.state_file, &backup_file)
                .context("Failed to create state backup")?;
        }

        // Atomic rename
        fs::rename(&temp_file, &self.state_file)
            .context("Failed to atomically rename state file")?;

        // Also save to a height-specific file for extra safety
        let height_file = self.config.base_dir.join(format!("state_height_{}.json", state.height));
        fs::copy(&self.state_file, &height_file)
            .context("Failed to create height-specific state backup")?;

        println!("State saved successfully at height {} (with backups)", state.height);
        Ok(())
    }

    /// Load state from disk with fallback to backups
    pub fn load_state(&self) -> Result<Option<State>> {
        // Try loading from main state file first
        if self.state_file.exists() {
            match self.try_load_state_file(&self.state_file) {
                Ok(state) => {
                    println!("State loaded successfully from main file at height {}", state.height);
                    return Ok(Some(state));
                }
                Err(e) => {
                    println!("Failed to load main state file: {}, trying backups...", e);
                }
            }
        }

        // Try loading from backup file
        let backup_file = self.state_file.with_extension("backup");
        if backup_file.exists() {
            match self.try_load_state_file(&backup_file) {
                Ok(state) => {
                    println!("State loaded successfully from backup file at height {}", state.height);
                    return Ok(Some(state));
                }
                Err(e) => {
                    println!("Failed to load backup state file: {}, trying checkpoints...", e);
                }
            }
        }

        // Try loading from latest checkpoint
        if let Ok(Some(state)) = self.load_from_checkpoint() {
            println!("State loaded successfully from checkpoint at height {}", state.height);
            return Ok(Some(state));
        }

        // Try loading from height-specific files (find the latest)
        if let Ok(Some(state)) = self.load_from_latest_height_file() {
            println!("State loaded successfully from height file at height {}", state.height);
            return Ok(Some(state));
        }

        println!("No valid state found, starting fresh");
        Ok(None)
    }

    /// Try to load state from a specific file
    fn try_load_state_file(&self, path: &Path) -> Result<State> {
        let file = File::open(path)
            .context("Failed to open state file")?;
        
        let reader = BufReader::new(file);
        let state: State = serde_json::from_reader(reader)
            .context("Failed to deserialize state")?;

        Ok(state)
    }

    /// Load state from the latest height-specific file
    fn load_from_latest_height_file(&self) -> Result<Option<State>> {
        let mut height_files = Vec::new();
        
        for entry in fs::read_dir(&self.config.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with("state_height_") && name_str.ends_with(".json") {
                        height_files.push(path);
                    }
                }
            }
        }

        if height_files.is_empty() {
            return Ok(None);
        }

        // Sort by height (extract height from filename)
        height_files.sort_by(|a, b| {
            let a_name = a.file_name().unwrap().to_str().unwrap();
            let b_name = b.file_name().unwrap().to_str().unwrap();
            let a_height = a_name.trim_start_matches("state_height_").trim_end_matches(".json");
            let b_height = b_name.trim_start_matches("state_height_").trim_end_matches(".json");
            a_height.parse::<u64>().unwrap_or(0).cmp(&b_height.parse::<u64>().unwrap_or(0))
        });

        // Load the latest height file
        let latest_file = height_files.last().unwrap();
        match self.try_load_state_file(latest_file) {
            Ok(state) => Ok(Some(state)),
            Err(e) => {
                println!("Failed to load height file {:?}: {}", latest_file, e);
                Ok(None)
            }
        }
    }

    /// Create a checkpoint of the current state
    pub fn create_checkpoint(&self, state: &State, block: &Block) -> Result<()> {
        let current_height = state.height;
        let last_checkpoint = *self.last_checkpoint.read();
        
        // Check if we need to create a checkpoint
        if current_height < last_checkpoint + self.config.checkpoint_interval {
            return Ok(());
        }

        let checkpoint_file = self.checkpoints_dir.join(format!("checkpoint_{:016x}.json", current_height));
        let temp_file = checkpoint_file.with_extension("tmp");

        // Create checkpoint data
        let checkpoint = Checkpoint {
            height: current_height,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            state: state.clone(),
            block_header: block.header.clone(),
        };

        // Write checkpoint to temporary file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_file)
            .context("Failed to create checkpoint file")?;
        
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &checkpoint)
            .context("Failed to serialize checkpoint")?;
        writer.flush().context("Failed to flush checkpoint file")?;
        drop(writer);

        // Atomic rename
        fs::rename(&temp_file, &checkpoint_file)
            .context("Failed to atomically rename checkpoint file")?;

        // Update last checkpoint
        *self.last_checkpoint.write() = current_height;

        // Clean up old checkpoints
        self.cleanup_old_checkpoints()?;

        println!("Checkpoint created at height {}", current_height);
        Ok(())
    }

    /// Load state from the latest checkpoint
    pub fn load_from_checkpoint(&self) -> Result<Option<State>> {
        let mut checkpoints = Vec::new();
        
        // Find all checkpoint files
        for entry in fs::read_dir(&self.checkpoints_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(name) = path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with("checkpoint_") {
                                checkpoints.push(path);
                            }
                        }
                    }
                }
            }
        }

        if checkpoints.is_empty() {
            return Ok(None);
        }

        // Sort by height (filename contains hex height)
        checkpoints.sort_by(|a, b| {
            let a_name = a.file_name().unwrap().to_str().unwrap();
            let b_name = b.file_name().unwrap().to_str().unwrap();
            a_name.cmp(b_name)
        });

        // Load the latest checkpoint
        let latest_checkpoint = checkpoints.last().unwrap();
        let file = File::open(latest_checkpoint)
            .context("Failed to open checkpoint file")?;
        
        let reader = BufReader::new(file);
        let checkpoint: Checkpoint = serde_json::from_reader(reader)
            .context("Failed to deserialize checkpoint")?;

        println!("Loaded checkpoint from height {}", checkpoint.height);
        Ok(Some(checkpoint.state))
    }

    /// Index a transaction for efficient querying
    pub fn index_transaction(&self, tx_hash: H256, block_height: u64, tx_index: usize) -> Result<()> {
        if !self.config.enable_indexing {
            return Ok(());
        }

        let index_file = self.index_dir.join("transactions.jsonl");
        let temp_file = index_file.with_extension("tmp");

        let index_entry = TransactionIndex {
            tx_hash: hex::encode(tx_hash),
            block_height,
            tx_index,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Append to index file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&temp_file)
            .context("Failed to open transaction index file")?;
        
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{}", serde_json::to_string(&index_entry)?)
            .context("Failed to write transaction index")?;
        writer.flush().context("Failed to flush index file")?;
        drop(writer);

        // Atomic rename
        fs::rename(&temp_file, &index_file)
            .context("Failed to atomically rename index file")?;

        Ok(())
    }

    /// Find transaction by hash
    pub fn find_transaction(&self, tx_hash: &str) -> Result<Option<TransactionIndex>> {
        if !self.config.enable_indexing {
            return Ok(None);
        }

        let index_file = self.index_dir.join("transactions.jsonl");
        if !index_file.exists() {
            return Ok(None);
        }

        let file = File::open(&index_file)
            .context("Failed to open transaction index file")?;
        
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.context("Failed to read index line")?;
            if let Ok(entry) = serde_json::from_str::<TransactionIndex>(&line) {
                if entry.tx_hash == tx_hash {
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }

    /// Create a backup of all data with enhanced persistence
    pub fn create_backup(&self) -> Result<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let last_backup = *self.last_backup.read();
        if current_time < last_backup + self.config.backup_interval_secs {
            return Ok(());
        }

        let backup_name = format!("backup_{}", current_time);
        let backup_path = self.backup_dir.join(&backup_name);
        
        // Create backup directory
        fs::create_dir_all(&backup_path)?;

        // Copy all important files with enhanced error handling
        let files_to_backup = [
            "state.json",
            "state.json.backup",
            "blocks",
            "checkpoints",
            "index",
        ];

        for file_name in &files_to_backup {
            let source = self.config.base_dir.join(file_name);
            let dest = backup_path.join(file_name);
            
            if source.exists() {
                if source.is_dir() {
                    copy_dir_recursive(&source, &dest)?;
                } else {
                    fs::copy(&source, &dest)?;
                }
            }
        }

        // Also backup height-specific state files
        for entry in fs::read_dir(&self.config.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with("state_height_") && name_str.ends_with(".json") {
                        let dest = backup_path.join(name_str);
                        fs::copy(&path, &dest)?;
                    }
                }
            }
        }

        // Create a manifest file for this backup
        let manifest = BackupManifest {
            timestamp: current_time,
            backup_name: backup_name.clone(),
            files_backed_up: files_to_backup.len(),
            blockchain_height: self.get_current_height_from_state()?,
        };

        let manifest_file = backup_path.join("manifest.json");
        let manifest_content = serde_json::to_string_pretty(&manifest)?;
        fs::write(manifest_file, manifest_content)?;

        // Update last backup time
        *self.last_backup.write() = current_time;

        // Clean up old backups
        self.cleanup_old_backups()?;

        println!("Enhanced backup created: {} (height: {})", backup_name, manifest.blockchain_height);
        Ok(())
    }

    /// Get current height from state file
    fn get_current_height_from_state(&self) -> Result<u64> {
        if let Ok(Some(state)) = self.load_state() {
            Ok(state.height)
        } else {
            Ok(0)
        }
    }

    /// Save a block with enhanced persistence
    pub fn save_block(&self, block: &Block) -> Result<()> {
        let fname = format!("{:016x}.json", block.header.height);
        let block_file = self.config.base_dir.join("blocks").join(&fname);
        let temp_file = block_file.with_extension("tmp");
        let backup_file = block_file.with_extension("backup");

        // Create blocks directory if it doesn't exist
        fs::create_dir_all(self.config.base_dir.join("blocks"))?;

        // Write to temporary file first
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_file)
            .context("Failed to create temporary block file")?;
        
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, block)
            .context("Failed to serialize block")?;
        writer.flush().context("Failed to flush block file")?;
        drop(writer);

        // Create backup of existing block if it exists
        if block_file.exists() {
            fs::copy(&block_file, &backup_file)
                .context("Failed to create block backup")?;
        }

        // Atomic rename
        fs::rename(&temp_file, &block_file)
            .context("Failed to atomically rename block file")?;

        // Also save to a compressed backup
        let compressed_file = self.config.base_dir.join("blocks").join(format!("{:016x}.json.gz", block.header.height));
        if self.config.enable_compression {
            // Note: In a real implementation, you'd use a compression library like flate2
            // For now, we'll just copy the file
            fs::copy(&block_file, &compressed_file)
                .context("Failed to create compressed block backup")?;
        }

        println!("Block {} saved with enhanced persistence", block.header.height);
        Ok(())
    }

    /// Clean up old checkpoints
    fn cleanup_old_checkpoints(&self) -> Result<()> {
        let mut checkpoints = Vec::new();
        
        for entry in fs::read_dir(&self.checkpoints_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(name) = path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with("checkpoint_") {
                                checkpoints.push(path);
                            }
                        }
                    }
                }
            }
        }

        if checkpoints.len() <= self.config.max_checkpoints {
            return Ok(());
        }

        // Sort by height and remove oldest
        checkpoints.sort_by(|a, b| {
            let a_name = a.file_name().unwrap().to_str().unwrap();
            let b_name = b.file_name().unwrap().to_str().unwrap();
            a_name.cmp(b_name)
        });

        let to_remove = checkpoints.len() - self.config.max_checkpoints;
        for checkpoint in checkpoints.iter().take(to_remove) {
            fs::remove_file(checkpoint)?;
        }

        println!("Cleaned up {} old checkpoints", to_remove);
        Ok(())
    }

    /// Clean up old backups
    fn cleanup_old_backups(&self) -> Result<()> {
        let max_backups = 5; // Keep last 5 backups
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str.starts_with("backup_") {
                            backups.push(path);
                        }
                    }
                }
            }
        }

        if backups.len() <= max_backups {
            return Ok(());
        }

        // Sort by timestamp and remove oldest
        backups.sort_by(|a, b| {
            let a_name = a.file_name().unwrap().to_str().unwrap();
            let b_name = b.file_name().unwrap().to_str().unwrap();
            a_name.cmp(b_name)
        });

        let to_remove = backups.len() - max_backups;
        for backup in backups.iter().take(to_remove) {
            fs::remove_dir_all(backup)?;
        }

        println!("Cleaned up {} old backups", to_remove);
        Ok(())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let mut stats = StorageStats::default();
        
        // Count blocks
        let blocks_dir = self.config.base_dir.join("blocks");
        if blocks_dir.exists() {
            stats.block_count = fs::read_dir(&blocks_dir)?.count();
        }

        // Count checkpoints
        if self.checkpoints_dir.exists() {
            stats.checkpoint_count = fs::read_dir(&self.checkpoints_dir)?.count();
        }

        // Count backups
        if self.backup_dir.exists() {
            stats.backup_count = fs::read_dir(&self.backup_dir)?.count();
        }

        // Count transactions in index
        let index_file = self.index_dir.join("transactions.jsonl");
        if index_file.exists() {
            let file = File::open(&index_file)?;
            let reader = BufReader::new(file);
            stats.total_transactions = reader.lines().count();
        }

        // Calculate total size
        stats.total_size = self.calculate_directory_size(&self.config.base_dir)?;

        // Set timing information
        stats.last_backup_time = *self.last_backup.read();
        stats.last_checkpoint_height = *self.last_checkpoint.read();

        // Determine storage health
        stats.storage_health = if stats.block_count > 0 && stats.checkpoint_count > 0 {
            "Healthy".to_string()
        } else if stats.block_count > 0 {
            "Warning: No checkpoints".to_string()
        } else {
            "New: No data yet".to_string()
        };

        Ok(stats)
    }

    /// Calculate directory size recursively
    fn calculate_directory_size(&self, path: &Path) -> Result<u64> {
        let mut size = 0;
        
        if path.is_file() {
            size += fs::metadata(path)?.len();
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                size += self.calculate_directory_size(&entry.path())?;
            }
        }
        
        Ok(size)
    }
}

/// Checkpoint data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Checkpoint {
    height: u64,
    timestamp: u64,
    state: State,
    block_header: BlockHeader,
}

/// Transaction index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIndex {
    pub tx_hash: String,
    pub block_height: u64,
    pub tx_index: usize,
    pub timestamp: u64,
}

/// Storage statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub block_count: usize,
    pub checkpoint_count: usize,
    pub backup_count: usize,
    pub total_size: u64,
    pub total_transactions: usize,
    pub last_backup_time: u64,
    pub last_checkpoint_height: u64,
    pub storage_health: String,
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_file() {
        fs::copy(src, dst)?;
    } else if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            copy_dir_recursive(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

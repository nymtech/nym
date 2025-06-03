-- Migration: Add simulated rewarding system tables
-- This migration adds support for simulated reward calculations to compare
-- old (24h cache-based) vs new (1h route-based) rewarding methodologies

-- Simulated reward epochs track each simulation run
CREATE TABLE IF NOT EXISTS simulated_reward_epochs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_id INTEGER NOT NULL,
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'comparison')),
    start_timestamp INTEGER NOT NULL,
    end_timestamp INTEGER NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Node performance data calculated from different methodologies
CREATE TABLE IF NOT EXISTS simulated_node_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    node_type TEXT NOT NULL CHECK (node_type IN ('mixnode', 'gateway')), 
    identity_key TEXT,
    reliability_score REAL NOT NULL CHECK (reliability_score >= 0.0 AND reliability_score <= 100.0),
    positive_samples INTEGER NOT NULL DEFAULT 0,
    negative_samples INTEGER NOT NULL DEFAULT 0,
    final_fail_sequence INTEGER NOT NULL DEFAULT 0,
    work_factor REAL CHECK (work_factor >= 0.0 AND work_factor <= 1.0),
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new')),
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Simulated reward calculations with full breakdown
CREATE TABLE IF NOT EXISTS simulated_rewards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    node_type TEXT NOT NULL CHECK (node_type IN ('mixnode', 'gateway')),
    calculated_reward_amount REAL NOT NULL CHECK (calculated_reward_amount >= 0.0),
    reward_currency TEXT NOT NULL DEFAULT 'nym',
    performance_component REAL NOT NULL CHECK (performance_component >= 0.0 AND performance_component <= 100.0),
    work_component REAL NOT NULL CHECK (work_component >= 0.0 AND work_component <= 1.0),
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new')),
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Route analysis metadata for each simulation run
CREATE TABLE IF NOT EXISTS simulated_route_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new')),
    total_routes_analyzed INTEGER NOT NULL DEFAULT 0,
    successful_routes INTEGER NOT NULL DEFAULT 0,
    failed_routes INTEGER NOT NULL DEFAULT 0,
    average_route_reliability REAL CHECK (average_route_reliability >= 0.0 AND average_route_reliability <= 100.0),
    time_window_hours INTEGER NOT NULL DEFAULT 1, -- New method uses 1 hour, old uses 24
    analysis_parameters TEXT, -- JSON with additional analysis configuration
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_epoch_id ON simulated_reward_epochs(epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_calculation_method ON simulated_reward_epochs(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_created_at ON simulated_reward_epochs(created_at);

CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_simulated_epoch_id ON simulated_node_performance(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_node_id ON simulated_node_performance(node_id);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_calculation_method ON simulated_node_performance(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_node_type ON simulated_node_performance(node_type);

CREATE INDEX IF NOT EXISTS idx_simulated_rewards_simulated_epoch_id ON simulated_rewards(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_rewards_node_id ON simulated_rewards(node_id);
CREATE INDEX IF NOT EXISTS idx_simulated_rewards_calculation_method ON simulated_rewards(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_rewards_node_type ON simulated_rewards(node_type);

CREATE INDEX IF NOT EXISTS idx_simulated_route_analysis_simulated_epoch_id ON simulated_route_analysis(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_route_analysis_calculation_method ON simulated_route_analysis(calculation_method);
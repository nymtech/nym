-- Migration: Add performance methodology comparison tables
-- This migration adds support for comparing performance calculation methodologies:
-- old (24h cache-based) vs new (1h route-based) without affecting actual rewards

-- Simulated reward epochs track each simulation run
CREATE TABLE IF NOT EXISTS simulated_reward_epochs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_id INTEGER NOT NULL,
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'comparison', 'test')),
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
    work_factor REAL CHECK (work_factor >= 0.0 AND work_factor <= 1.0),
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'test')),
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Performance comparison data for analyzing methodology differences
CREATE TABLE IF NOT EXISTS simulated_performance_comparisons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    node_type TEXT NOT NULL CHECK (node_type IN ('mixnode', 'gateway')),
    -- Performance scores from each methodology
    performance_score REAL NOT NULL CHECK (performance_score >= 0.0 AND performance_score <= 100.0),
    work_factor REAL NOT NULL CHECK (work_factor >= 0.0),
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'test')),
    -- Additional metrics for analysis
    positive_samples INTEGER DEFAULT 0,
    negative_samples INTEGER DEFAULT 0,
    route_success_rate REAL CHECK (route_success_rate >= 0.0 AND route_success_rate <= 100.0),
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Route analysis metadata for each simulation run
CREATE TABLE IF NOT EXISTS simulated_route_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'test')),
    total_routes_analyzed INTEGER NOT NULL DEFAULT 0,
    successful_routes INTEGER NOT NULL DEFAULT 0,
    failed_routes INTEGER NOT NULL DEFAULT 0,
    average_route_reliability REAL CHECK (average_route_reliability >= 0.0 AND average_route_reliability <= 100.0),
    time_window_hours INTEGER NOT NULL DEFAULT 1, -- New method uses 1 hour, old uses 24
    analysis_parameters TEXT, -- JSON with additional analysis configuration
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE
);

-- Performance rankings and comparison analytics
CREATE TABLE IF NOT EXISTS simulated_performance_rankings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    simulated_epoch_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    calculation_method TEXT NOT NULL CHECK (calculation_method IN ('old', 'new', 'test')),
    performance_rank INTEGER NOT NULL,
    performance_percentile REAL NOT NULL CHECK (performance_percentile >= 0.0 AND performance_percentile <= 100.0),
    calculated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (simulated_epoch_id) REFERENCES simulated_reward_epochs(id) ON DELETE CASCADE,
    UNIQUE(simulated_epoch_id, node_id, calculation_method)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_epoch_id ON simulated_reward_epochs(epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_calculation_method ON simulated_reward_epochs(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_reward_epochs_created_at ON simulated_reward_epochs(created_at);

CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_simulated_epoch_id ON simulated_node_performance(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_node_id ON simulated_node_performance(node_id);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_calculation_method ON simulated_node_performance(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_node_performance_node_type ON simulated_node_performance(node_type);

CREATE INDEX IF NOT EXISTS idx_simulated_performance_comparisons_simulated_epoch_id ON simulated_performance_comparisons(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_comparisons_node_id ON simulated_performance_comparisons(node_id);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_comparisons_calculation_method ON simulated_performance_comparisons(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_comparisons_node_type ON simulated_performance_comparisons(node_type);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_comparisons_performance_score ON simulated_performance_comparisons(performance_score);

CREATE INDEX IF NOT EXISTS idx_simulated_route_analysis_simulated_epoch_id ON simulated_route_analysis(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_route_analysis_calculation_method ON simulated_route_analysis(calculation_method);

CREATE INDEX IF NOT EXISTS idx_simulated_performance_rankings_simulated_epoch_id ON simulated_performance_rankings(simulated_epoch_id);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_rankings_node_id ON simulated_performance_rankings(node_id);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_rankings_calculation_method ON simulated_performance_rankings(calculation_method);
CREATE INDEX IF NOT EXISTS idx_simulated_performance_rankings_performance_rank ON simulated_performance_rankings(performance_rank);
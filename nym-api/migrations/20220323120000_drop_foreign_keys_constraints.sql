/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE failed_mixnode_reward_chunk_new as SELECT * FROM failed_mixnode_reward_chunk;
DROP TABLE failed_mixnode_reward_chunk;
ALTER TABLE failed_mixnode_reward_chunk_new RENAME TO failed_mixnode_reward_chunk;

CREATE TABLE gateway_historical_uptime_new as SELECT * FROM gateway_historical_uptime;
DROP TABLE gateway_historical_uptime;
ALTER TABLE gateway_historical_uptime_new RENAME TO gateway_historical_uptime;

CREATE TABLE gateway_status_new as SELECT * FROM gateway_status;
DROP TABLE gateway_status;
ALTER TABLE gateway_status_new RENAME TO gateway_status;

CREATE TABLE mixnode_historical_uptime_new as SELECT * FROM mixnode_historical_uptime;
DROP TABLE mixnode_historical_uptime;
ALTER TABLE mixnode_historical_uptime_new RENAME TO mixnode_historical_uptime;

CREATE TABLE mixnode_status_new as SELECT * FROM mixnode_status;
DROP TABLE mixnode_status;
ALTER TABLE mixnode_status_new RENAME TO mixnode_status;

CREATE TABLE possibly_unrewarded_mixnode_new as SELECT * FROM possibly_unrewarded_mixnode;
DROP TABLE possibly_unrewarded_mixnode;
ALTER TABLE possibly_unrewarded_mixnode_new RENAME TO possibly_unrewarded_mixnode;

CREATE TABLE rewarding_report_new as SELECT * FROM rewarding_report;
DROP TABLE rewarding_report;
ALTER TABLE rewarding_report_new RENAME TO rewarding_report;

CREATE TABLE testing_route_new as SELECT * FROM testing_route;
DROP TABLE testing_route;
ALTER TABLE testing_route_new RENAME TO testing_route;

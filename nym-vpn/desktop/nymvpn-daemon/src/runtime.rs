// Copyright (C) 2023 Nym Technologies S.A., GPL-3.0
// Copyright (C) 2022 Mullvad VPN AB, GPL-3.0
pub fn create_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
}

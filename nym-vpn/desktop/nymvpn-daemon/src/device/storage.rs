use std::net::Ipv4Addr;

use chrono::Utc;
use nymvpn_migration::{
    sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QuerySelect, Set},
    DbErr,
};
use nymvpn_types::{
    device::{DeviceDetails, ANDROID, IOS, LINUX, MACOS, WINDOWS},
    nymvpn_server::DeviceType,
};
use uuid::Uuid;

use crate::device::name::{device_name, device_version};

#[derive(Clone)]
pub struct DeviceStorage {
    db: DatabaseConnection,
}

fn create_new() -> Result<DeviceDetails, String> {
    let device_type = match std::env::consts::OS {
        LINUX => DeviceType::Linux,
        MACOS => DeviceType::MacOS,
        WINDOWS => DeviceType::Windows,
        ANDROID => DeviceType::Android,
        IOS => DeviceType::IOS,
        os => Err(format!("OS not supported: {os}"))?,
    };
    let arch = std::env::consts::ARCH;
    tracing::info!("detected device type: {device_type}, arch : {arch}");
    Ok(DeviceDetails {
        name: device_name(),
        version: device_version(),
        arch: arch.into(),
        unique_id: uuid::Uuid::new_v4(),
        device_type,
        wireguard_meta: Default::default(),
        created_at: Utc::now(),
    })
}

impl DeviceStorage {
    pub fn new(db: DatabaseConnection) -> DeviceStorage {
        Self { db }
    }

    pub async fn init(&self) -> Result<DeviceDetails, String> {
        tracing::info!("Initializing device");
        let device = nymvpn_entity::device::Entity::find()
            .one(&self.db)
            .await
            .map_err(|e| format!("unable to get device details: {e}"))?;

        if let Some(device) = device {
            tracing::info!("device already initialized");
            let device_details = device.try_into()?;
            tracing::info!("found: {device_details}");
            return Ok(device_details);
        }

        // No device found create one
        tracing::info!("creating new device");
        let device_details = create_new()?;

        // save to DB
        let device: nymvpn_entity::device::Model = device_details.clone().into();
        let device: nymvpn_entity::device::ActiveModel = device.into();

        let _ = device
            .insert(&self.db)
            .await
            .map_err(|e| format!("failed to save device detail {e}"))?;

        tracing::info!("new device saved: {device_details}");

        Ok(device_details)
    }

    pub async fn reinitialize(&self, reason: &str) -> Result<(), DbErr> {
        tracing::info!("reinitializing device: {reason}");
        let _ = nymvpn_entity::device::Entity::delete_many()
            .exec(&self.db)
            .await?;
        let _ = self.init().await;
        Ok(())
    }

    // Assumes device was initialized successfully
    pub async fn get_device_unique_id(&self) -> Result<Uuid, DbErr> {
        let id: (String,) = nymvpn_entity::device::Entity::find()
            .select_only()
            .column(nymvpn_entity::device::Column::UniqueId)
            .into_tuple()
            .one(&self.db)
            .await?
            .unwrap();

        Ok(Uuid::parse_str(&id.0).unwrap())
    }

    pub async fn get_device(&self) -> Result<Option<DeviceDetails>, DbErr> {
        let device = nymvpn_entity::device::Entity::find().one(&self.db).await?;

        if let Some(device) = device {
            let device_details = device.try_into().map_err(|e| DbErr::Custom(e))?;
            return Ok(Some(device_details));
        }

        Ok(None)
    }

    pub async fn update_ipv4_address(
        &self,
        unique_id: Uuid,
        ipv4_address: Ipv4Addr,
    ) -> Result<DeviceDetails, DbErr> {
        let device = nymvpn_entity::device::Entity::find_by_id(unique_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "device with unique id {unique_id} not found"
            )))?;

        let mut device: nymvpn_entity::device::ActiveModel = device.into();
        device.ipv4_address = Set(Some(ipv4_address.to_string()));

        let device = device.update(&self.db).await?;

        Ok(device.try_into().map_err(|e| DbErr::Custom(e))?)
    }
}

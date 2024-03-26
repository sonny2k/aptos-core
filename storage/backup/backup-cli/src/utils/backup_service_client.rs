// Copyright (c) Aptos Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::utils::error_notes::ErrorNotes;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_db::backup::backup_handler::DbState;
use aptos_types::transaction::Version;
use clap::Parser;
use futures::TryStreamExt;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::Duration,
};
use tokio_io_timeout::TimeoutReader;
use tokio_util::compat::FuturesAsyncReadCompatExt;

#[derive(Parser)]
pub struct BackupServiceClientOpt {
    #[clap(
        long = "backup-service-address",
        default_value = "http://localhost:6186",
        help = "Backup service address. By default a Aptos Node runs the backup service serving \
        on tcp port 6186 to localhost only."
    )]
    pub address: String,
}

pub struct BackupServiceClient {
    address: String,
    client: reqwest::Client,
}

impl BackupServiceClient {
    const TIMEOUT_SECS: u64 = 60;

    pub fn new_with_opt(opt: BackupServiceClientOpt) -> Self {
        Self::new(opt.address)
    }

    pub fn new(address: String) -> Self {
        Self {
            address,
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("Http client should build."),
        }
    }

    async fn get(&self, path: &str) -> Result<impl AsyncRead> {
        let url = format!("{}/{}", self.address, path);
        let timeout = Duration::from_secs(Self::TIMEOUT_SECS);
        let reader = tokio::time::timeout(timeout, self.client.get(&url).send())
            .await?
            .err_notes(&url)?
            .error_for_status()
            .err_notes(&url)?
            .bytes_stream()
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
            .into_async_read()
            .compat();

        // Adding the timeout here instead of on the response because we do use long living
        // connections. For example, we stream the entire state snapshot in one request.
        let mut reader_with_read_timeout = TimeoutReader::new(reader);
        reader_with_read_timeout.set_timeout(Some(timeout));

        Ok(Box::pin(reader_with_read_timeout))
    }

    pub async fn get_db_state(&self) -> Result<Option<DbState>> {
        let mut buf = Vec::new();
        self.get("db_state").await?.read_to_end(&mut buf).await?;
        Ok(bcs::from_bytes(&buf)?)
    }

    pub async fn get_account_range_proof(
        &self,
        key: HashValue,
        version: Version,
    ) -> Result<impl AsyncRead> {
        self.get(&format!("state_range_proof/{}/{:x}", version, key))
            .await
    }

    pub async fn get_state_snapshot(&self, version: Version) -> Result<impl AsyncRead> {
        self.get(&format!("state_snapshot/{}", version)).await
    }

    pub async fn get_state_root_proof(&self, version: Version) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.get(&format!("state_root_proof/{}", version))
            .await?
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }

    pub async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<impl AsyncRead> {
        self.get(&format!(
            "epoch_ending_ledger_infos/{}/{}",
            start_epoch, end_epoch
        ))
        .await
    }

    pub async fn get_transactions(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl AsyncRead> {
        self.get(&format!(
            "transactions/{}/{}",
            start_version, num_transactions
        ))
        .await
    }

    pub async fn get_transaction_range_proof(
        &self,
        first_version: Version,
        last_version: Version,
    ) -> Result<impl AsyncRead> {
        self.get(&format!(
            "transaction_range_proof/{}/{}",
            first_version, last_version,
        ))
        .await
    }
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::{
    peer::DisconnectReason,
    peer_manager::PeerManagerError,
    protocols::{
        direct_send::Message,
        rpc::{InboundRpcRequest, OutboundRpcRequest},
    },
    transport::{Connection, ConnectionMetadata},
    ProtocolId,
};
use aptos_config::network_id::NetworkId;
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::channel::oneshot;
use serde::Serialize;
use std::fmt;
use tokio::time::Instant;

/// A container holding messages with various metadata
#[derive(Clone, Debug)]
pub struct MessageWithMetadata {
    message: Message,
    latency_metadata: MessageLatencyMetadata,
}

impl MessageWithMetadata {
    /// Creates a new message with metadata
    pub fn new(message: Message, latency_metadata: MessageLatencyMetadata) -> Self {
        Self {
            message,
            latency_metadata,
        }
    }

    /// Creates a new message without latency metadata. This is only used for testing purposes.
    #[cfg(test)]
    pub fn new_without_metadata(message: Message) -> Self {
        Self {
            message,
            latency_metadata: MessageLatencyMetadata::new_empty(),
        }
    }

    /// Returns a reference to the message
    pub fn get_message(&self) -> &Message {
        &self.message
    }

    /// Returns the message latency metadata
    pub fn get_latency_metadata(&self) -> MessageLatencyMetadata {
        self.latency_metadata
    }
}

/// A struct holding simple latency metadata for each network message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageLatencyMetadata {
    serialization_start_time: Option<Instant>, // Time when the message started serialization
    dispatch_start_time: Option<Instant>, // Time when the message was first dispatched (after serialization)
}

impl MessageLatencyMetadata {
    /// Creates an empty message latency metadata
    pub fn new_empty() -> Self {
        Self {
            serialization_start_time: None,
            dispatch_start_time: None,
        }
    }

    /// Sets the serialization start time
    pub fn set_serialization_start_time(&mut self) {
        self.serialization_start_time = Some(Instant::now());
    }

    /// Sets the dispatch start time
    pub fn set_dispatch_start_time(&mut self) {
        self.dispatch_start_time = Some(Instant::now());
    }
}

impl Default for MessageLatencyMetadata {
    fn default() -> Self {
        Self::new_empty()
    }
}

/// Request received by PeerManager from upstream actors.
#[derive(Debug, Serialize)]
pub enum PeerManagerRequest {
    /// Send an RPC request to a remote peer.
    SendRpc(PeerId, #[serde(skip)] OutboundRpcRequest),
    /// Fire-and-forget style message send to a remote peer.
    SendDirectSend(PeerId, #[serde(skip)] MessageWithMetadata),
}

impl PeerManagerRequest {
    /// Creates and returns a new direct send message request
    pub fn new_direct_send(
        peer_id: PeerId,
        protocol_id: ProtocolId,
        mdata: Bytes,
        latency_metadata: MessageLatencyMetadata,
    ) -> Self {
        // Create the message with metadata
        let message = Message { protocol_id, mdata };
        let message_with_metadata = MessageWithMetadata::new(message, latency_metadata);

        // Return the direct send request
        Self::SendDirectSend(peer_id, message_with_metadata)
    }
}

/// Notifications sent by PeerManager to upstream actors.
#[derive(Debug)]
pub enum PeerManagerNotification {
    /// A new RPC request has been received from a remote peer.
    RecvRpc(PeerId, InboundRpcRequest),
    /// A new message has been received from a remote peer.
    RecvMessage(PeerId, Message),
}

impl PeerManagerNotification {
    /// Returns the peer ID of the notification
    pub fn get_peer_id(&self) -> PeerId {
        match self {
            PeerManagerNotification::RecvRpc(peer_id, _) => *peer_id,
            PeerManagerNotification::RecvMessage(peer_id, _) => *peer_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ConnectionRequest {
    DialPeer(
        PeerId,
        NetworkAddress,
        #[serde(skip)] oneshot::Sender<Result<(), PeerManagerError>>,
    ),
    DisconnectPeer(
        PeerId,
        #[serde(skip)] oneshot::Sender<Result<(), PeerManagerError>>,
    ),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub enum ConnectionNotification {
    /// Connection with a new peer has been established.
    NewPeer(ConnectionMetadata, NetworkId),
    /// Connection to a peer has been terminated. This could have been triggered from either end.
    LostPeer(ConnectionMetadata, NetworkId),
}

impl fmt::Debug for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ConnectionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionNotification::NewPeer(metadata, network_id) => {
                write!(f, "[{},{}]", metadata, network_id)
            },
            ConnectionNotification::LostPeer(metadata, network_id) => {
                write!(f, "[{},{}]", metadata, network_id)
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub enum TransportNotification<TSocket> {
    NewConnection(#[serde(skip)] Connection<TSocket>),
    Disconnected(ConnectionMetadata, DisconnectReason),
}

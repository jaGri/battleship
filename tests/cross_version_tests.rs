use battleship::transport::in_memory::InMemoryTransport;
use battleship::transport::tcp::TcpTransport;
use battleship::transport::Transport;
use battleship::protocol::{Message, PROTOCOL_VERSION};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_version_mismatch_reject() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Player 1 sends handshake with version 1
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version } if version == PROTOCOL_VERSION));
    
    // Player 2 responds with incompatible version
    let wrong_version = PROTOCOL_VERSION + 10;
    t2.send(Message::HandshakeAck {
        version: wrong_version,
    })
    .await
    .unwrap();
    
    // Player 1 receives mismatched version
    let reply = t1.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, wrong_version);
        assert_ne!(version, PROTOCOL_VERSION);
    } else {
        panic!("Expected HandshakeAck");
    }
}

#[tokio::test]
async fn test_old_client_new_server() {
    let (mut old_client, mut new_server) = InMemoryTransport::pair();
    
    let old_version = PROTOCOL_VERSION.saturating_sub(1);
    
    // Old client sends old version
    old_client.send(Message::Handshake {
        version: old_version,
    })
    .await
    .unwrap();
    
    let msg = new_server.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version } if version == old_version));
    
    // New server acknowledges with its version
    new_server.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    // Old client receives version it doesn't support
    let reply = old_client.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, PROTOCOL_VERSION);
        assert_ne!(version, old_version);
    }
}

#[tokio::test]
async fn test_new_client_old_server() {
    let (mut new_client, mut old_server) = InMemoryTransport::pair();
    
    let old_version = PROTOCOL_VERSION.saturating_sub(1);
    
    // New client sends current version
    new_client.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let msg = old_server.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version } if version == PROTOCOL_VERSION));
    
    // Old server responds with old version
    old_server.send(Message::HandshakeAck {
        version: old_version,
    })
    .await
    .unwrap();
    
    // New client receives incompatible version
    let reply = new_client.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, old_version);
        assert_ne!(version, PROTOCOL_VERSION);
    }
}

#[tokio::test]
async fn test_future_version() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    let future_version = PROTOCOL_VERSION + 100;
    
    // Future client
    t1.send(Message::Handshake {
        version: future_version,
    })
    .await
    .unwrap();
    
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version } if version == future_version));
    
    // Current server rejects
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let reply = t1.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_ne!(version, future_version);
    }
}

#[tokio::test]
async fn test_version_zero() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Send version 0 (invalid)
    t1.send(Message::Handshake { version: 0 })
        .await
        .unwrap();
    
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version: 0 }));
    
    // Respond with current version
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let reply = t1.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, PROTOCOL_VERSION);
        assert_ne!(version, 0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_version_tcp() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let old_version = PROTOCOL_VERSION.saturating_sub(1);
    
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(socket);
        
        // Server with current version
        let msg = transport.recv().await.unwrap();
        assert!(matches!(msg, Message::Handshake { .. }));
        
        transport.send(Message::HandshakeAck {
            version: PROTOCOL_VERSION,
        })
        .await
        .unwrap();
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Client with old version
    transport.send(Message::Handshake {
        version: old_version,
    })
    .await?;
    
    let reply = transport.recv().await?;
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, PROTOCOL_VERSION);
        assert_ne!(version, old_version);
    }
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_version_handshakes() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(socket);
        
        // Receive multiple handshakes
        for _ in 0..3 {
            let msg = transport.recv().await.unwrap();
            if let Message::Handshake { version: _ } = msg {
                // Always respond with current version
                transport.send(Message::HandshakeAck {
                    version: PROTOCOL_VERSION,
                })
                .await
                .unwrap();
            }
        }
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try handshake with different versions
    for offset in 0..3 {
        let version = PROTOCOL_VERSION.saturating_sub(offset);
        transport.send(Message::Handshake { version }).await?;
        
        let reply = transport.recv().await?;
        if let Message::HandshakeAck { version: ack_version } = reply {
            assert_eq!(ack_version, PROTOCOL_VERSION);
        }
    }
    
    server_task.await?;
    Ok(())
}

#[tokio::test]
async fn test_version_in_guess_message() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Complete handshake
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    t2.recv().await.unwrap();
    
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    t1.recv().await.unwrap();
    
    // Send guess with wrong version
    let wrong_version = PROTOCOL_VERSION + 5;
    t1.send(Message::Guess {
        version: wrong_version,
        seq: 0,
        x: 0,
        y: 0,
    })
    .await
    .unwrap();
    
    let msg = t2.recv().await.unwrap();
    if let Message::Guess { version, .. } = msg {
        assert_eq!(version, wrong_version);
        assert_ne!(version, PROTOCOL_VERSION);
    }
}

#[tokio::test]
async fn test_compatible_versions() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Both use same version
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version } if version == PROTOCOL_VERSION));
    
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    
    let reply = t1.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_eq!(version, PROTOCOL_VERSION);
    }
    
    // Now they should be able to exchange game messages
    t1.send(Message::Guess {
        version: PROTOCOL_VERSION,
        seq: 0,
        x: 5,
        y: 5,
    })
    .await
    .unwrap();
    
    let guess_msg = t2.recv().await.unwrap();
    assert!(matches!(guess_msg, Message::Guess { version, seq: 0, x: 5, y: 5 } if version == PROTOCOL_VERSION));
}

#[tokio::test]
async fn test_version_negotiation_rejection() {
    let (mut t1, mut t2) = InMemoryTransport::pair();

    // Attempt negotiation with mismatched versions
    let old_version = 0;
    t1.send(Message::Handshake { version: old_version }).await.unwrap();
    let msg = t2.recv().await.unwrap();

    if let Message::Handshake { version: client_version } = msg {
        // Server sees old version, responds with current version
        assert_eq!(client_version, old_version);

        // Send rejection by indicating supported version
        t2.send(Message::HandshakeAck { version: PROTOCOL_VERSION })
            .await
            .unwrap();

        let reply = t1.recv().await.unwrap();
        if let Message::HandshakeAck { version: server_version } = reply {
            // Client sees server doesn't support its version
            assert_ne!(client_version, server_version);
            assert_eq!(server_version, PROTOCOL_VERSION);
        }
    }
}

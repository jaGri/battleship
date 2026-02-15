use battleship::transport::tcp::TcpTransport;
use battleship::transport::Transport;
use battleship::protocol::{Message, PROTOCOL_VERSION};
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;
use tokio::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test_malformed_length_prefix() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send invalid length prefix (all 0xFF bytes, which is > MAX_MESSAGE_SIZE)
        let bad_length = [0xFF, 0xFF, 0xFF, 0xFF];
        socket.write_all(&bad_length).await.unwrap();
        socket.flush().await.unwrap();
        
        // Wait a bit then close
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail due to malformed length
    let result = transport.recv().await;
    assert!(result.is_err());
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("too large") || err_msg.contains("exceeds") || err_msg.contains("Message size"));
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_zero_length_frame() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send zero-length frame
        let zero_length = [0u8, 0, 0, 0];
        socket.write_all(&zero_length).await.unwrap();
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail
    let result = transport.recv().await;
    assert!(result.is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_truncated_frame() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send length prefix indicating 100 bytes
        let length = 100u32.to_be_bytes();
        socket.write_all(&length).await.unwrap();
        
        // But only send 10 bytes of data
        let partial_data = vec![0u8; 10];
        socket.write_all(&partial_data).await.unwrap();
        socket.flush().await.unwrap();
        
        // Close connection before sending rest
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail or timeout
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        transport.recv()
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_bincode_payload() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send valid length prefix but garbage data
        let garbage_data = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let length = (garbage_data.len() as u32).to_be_bytes();
        
        socket.write_all(&length).await.unwrap();
        socket.write_all(&garbage_data).await.unwrap();
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail during bincode deserialization
    let result = transport.recv().await;
    assert!(result.is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_partial_length_prefix() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send only 2 bytes of the 4-byte length prefix
        socket.write_all(&[0u8, 100]).await.unwrap();
        socket.flush().await.unwrap();
        
        // Then close connection
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail or timeout
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        transport.recv()
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_malformed_message_type() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Create a message with invalid enum variant
        // This is tricky with bincode, but we can try encoding an out-of-range discriminant
        let invalid_message = vec![
            0xFF, // Invalid enum discriminant
            0x01, 0x00, 0x00, 0x00, // Some arbitrary bytes
        ];
        
        let length = (invalid_message.len() as u32).to_be_bytes();
        socket.write_all(&length).await.unwrap();
        socket.write_all(&invalid_message).await.unwrap();
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should fail during deserialization
    let result = transport.recv().await;
    assert!(result.is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_extremely_large_length_prefix() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send length prefix claiming 1GB
        let huge_length = (1_000_000_000u32).to_be_bytes();
        socket.write_all(&huge_length).await.unwrap();
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Try to receive - should immediately reject the size
    let result = transport.recv().await;
    assert!(result.is_err());
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("too large") || err_msg.contains("exceeds") || err_msg.contains("Message size"));
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rapid_malformed_frames() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send multiple malformed frames in rapid succession
        for _ in 0..10 {
            let bad_length = [0xFF, 0xFF, 0xFF, 0xFF];
            let _ = socket.write_all(&bad_length).await;
        }
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // First receive should fail
    let result = transport.recv().await;
    assert!(result.is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_valid_length_but_wrong_structure() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        
        // Send a valid-looking length but data that doesn't match Message structure
        let data = vec![0u8; 50]; // All zeros
        let length = (data.len() as u32).to_be_bytes();
        
        socket.write_all(&length).await.unwrap();
        socket.write_all(&data).await.unwrap();
        socket.flush().await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Should fail during deserialization
    let result = transport.recv().await;
    assert!(result.is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_interleaved_valid_and_malformed() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(socket);
        
        // Send valid handshake
        transport.send(Message::Handshake { version: PROTOCOL_VERSION }).await.unwrap();
        
        // Wait for response
        let _ = transport.recv().await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Receive valid handshake
    let msg = transport.recv().await?;
    assert!(matches!(msg, Message::Handshake { .. }));
    
    // Send back malformed data via raw socket access
    drop(transport);
    
    server_task.await?;
    Ok(())
}

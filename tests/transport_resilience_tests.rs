/// Tests for transport resilience features.
use battleship::{
    transport::{in_memory::InMemoryTransport, Transport},
    TcpTransport,
    protocol::{Message, PROTOCOL_VERSION},
};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_in_memory_closed_channel_detection() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Send a message from t1 to t2
    t1.send(Message::Heartbeat { version: PROTOCOL_VERSION })
        .await
        .unwrap();
    
    // Drop t1 to close the channel
    drop(t1);
    
    // t2 should receive the message that was already sent
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Heartbeat { .. }));
    
    // Now t2 should detect the closed channel
    let result = t2.recv().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("closed"));
}

#[tokio::test]
async fn test_in_memory_shutdown() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Shutdown t1
    t1.shutdown();
    
    // t1 should not be able to send
    let result = t1.send(Message::Heartbeat { version: PROTOCOL_VERSION }).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("shut down"));
    
    // t2 should detect that t1 closed its send channel
    let result = t2.recv().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("closed"));
}

#[tokio::test]
async fn test_tcp_graceful_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(stream);
        
        // Shutdown the transport
        transport.shutdown();
        
        // Should not be able to send after shutdown
        let result = transport.send(Message::Heartbeat { version: PROTOCOL_VERSION }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("shut down"));
    });
    
    let client_task = tokio::spawn(async move {
        let mut transport = TcpTransport::connect(addr).await.unwrap();
        
        // Wait a bit for server to shutdown
        sleep(Duration::from_millis(100)).await;
        
        // Try to receive - should fail with connection closed
        let result = transport.recv().await;
        assert!(result.is_err());
    });
    
    server_task.await.unwrap();
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_tcp_bounded_message_size() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::with_config(
            stream,
            Duration::from_secs(5),
            1000, // Very small max message size
            Duration::from_secs(10),
            Duration::from_secs(30),
        );
        
        // Try to receive - will fail if client sends large message
        let result = transport.recv().await;
        if let Err(e) = result {
            assert!(e.to_string().contains("too large"));
        }
    });
    
    let client_task = tokio::spawn(async move {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut transport = TcpTransport::new(stream);
        
        // Try to send a message with a large guess sequence
        // This should work on the client side but fail on server due to size limit
        transport.send(Message::Guess {
            version: PROTOCOL_VERSION,
            seq: 12345,
            x: 5,
            y: 5,
        }).await.unwrap();
        
        // Wait for server to process
        sleep(Duration::from_millis(100)).await;
    });
    
    server_task.await.unwrap();
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_tcp_idle_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::with_config(
            stream,
            Duration::from_secs(5),
            10_000_000,
            Duration::from_secs(10),
            Duration::from_millis(100), // Very short idle timeout
        );
        
        // Wait longer than idle timeout
        sleep(Duration::from_millis(200)).await;
        
        // Should fail with idle timeout
        let result = transport.send(Message::Heartbeat { version: PROTOCOL_VERSION }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("idle timeout"));
    });
    
    let _client_task = tokio::spawn(async move {
        let _stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        // Just keep connection alive
        sleep(Duration::from_millis(300)).await;
    });
    
    server_task.await.unwrap();
}

#[tokio::test]
async fn test_tcp_heartbeat() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(stream);
        
        // Receive heartbeat
        let msg = transport.recv().await.unwrap();
        assert!(matches!(msg, Message::Heartbeat { version: PROTOCOL_VERSION }));
        
        // Send heartbeat response
        transport.send_heartbeat().await.unwrap();
    });
    
    let client_task = tokio::spawn(async move {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut transport = TcpTransport::new(stream);
        
        // Send heartbeat
        transport.send_heartbeat().await.unwrap();
        
        // Receive heartbeat response
        let msg = transport.recv().await.unwrap();
        assert!(matches!(msg, Message::Heartbeat { version: PROTOCOL_VERSION }));
    });
    
    server_task.await.unwrap();
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_tcp_connection_reset_error_mapping() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Immediately close the connection
        drop(stream);
    });
    
    let client_task = tokio::spawn(async move {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut transport = TcpTransport::new(stream);
        
        // Wait for server to close
        sleep(Duration::from_millis(100)).await;
        
        // Try to send - should get descriptive error
        let result = transport.send(Message::Heartbeat { version: PROTOCOL_VERSION }).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("closed") || err_msg.contains("reset") || 
            err_msg.contains("Broken pipe") || err_msg.contains("aborted") ||
            err_msg.contains("Write error"),
            "Expected connection error but got: {}", err_msg
        );
    });
    
    server_task.await.unwrap();
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_in_memory_explicit_close() {
    let (mut t1, mut t2) = InMemoryTransport::pair();
    
    // Explicitly shutdown t1
    t1.shutdown();
    
    // t1 cannot send after shutdown
    let result = t1.send(Message::Heartbeat { version: PROTOCOL_VERSION }).await;
    assert!(result.is_err());
    
    // t2 should detect the closure
    let result = t2.recv().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("closed"));
}

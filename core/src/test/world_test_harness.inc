--- for tests which need a mock world:

    let mut buffer: Vec<u8> = Vec::new();
    let mut mock_sock = std::io::Cursor::new(&mut buffer);
    let (tx, _) = tokio::sync::broadcast::channel::<crate::Broadcast>(16);
    let (world, plr) = get_operational_mock_world().await;
    plr.write().await.access = Access::Builder;

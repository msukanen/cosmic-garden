// for tests which need a mock world:
    let mut b: Vec<u8> = vec![];
    let mut s = std::io::Cursor::new(&mut b);
    let (w,c,p,d) = get_operational_mock_world().await;
    let jt = get_operational_mock_janitor!(c,w,d.0);
    let gt = get_operational_mock_life(c,w);
    let lt = get_operational_mock_librarian(c,w);

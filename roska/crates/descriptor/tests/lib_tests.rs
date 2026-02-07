use roska_descriptor::Depth;

#[test]
fn test_depth_ordering() {
    assert!(Depth::Overview < Depth::Structure);
    assert!(Depth::Structure < Depth::Detail);
    assert!(Depth::Detail < Depth::Body);
}

#[test]
fn test_depth_from_u8() {
    assert_eq!(Depth::from_u8(0), Depth::Overview);
    assert_eq!(Depth::from_u8(1), Depth::Structure);
    assert_eq!(Depth::from_u8(2), Depth::Detail);
    assert_eq!(Depth::from_u8(5), Depth::Body); // clamps to max
}

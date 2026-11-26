//! capture-core::link の単体テスト。

use capture_core::link::{Command, FrameKind, LineReader, LinkError, crc16, encode};

#[test]
fn crc16_ccitt_false_check_value_is_29b1() {
    assert_eq!(crc16(b"123456789"), 0x29B1);
}

#[test]
fn encode_writes_sync_kind_length_payload_and_crc() {
    let mut out = [0u8; 16];
    let n = encode(FrameKind::Error, &[0x03], &mut out);
    assert_eq!(n, 8);
    assert_eq!(&out[..6], &[0xA5, 0x5A, 0x7F, 0x01, 0x00, 0x03]);
    assert_eq!(
        u16::from_le_bytes([out[6], out[7]]),
        crc16(&[0x7F, 0x01, 0x00, 0x03])
    );
}

fn feed(r: &mut LineReader, bytes: &[u8]) -> Vec<Result<Command, LinkError>> {
    bytes.iter().filter_map(|&b| r.push(b)).collect()
}

#[test]
fn line_reader_recognizes_start_stop_and_info() {
    let mut r = LineReader::new();
    assert_eq!(
        feed(&mut r, b"START\nSTOP\nINFO\n"),
        vec![Ok(Command::Start), Ok(Command::Stop), Ok(Command::Info)]
    );
}

#[test]
fn line_reader_ignores_carriage_returns_and_empty_lines() {
    let mut r = LineReader::new();
    assert_eq!(feed(&mut r, b"\n\r\nSTART\r\n"), vec![Ok(Command::Start)]);
}

#[test]
fn line_reader_rejects_unknown_lines() {
    let mut r = LineReader::new();
    assert_eq!(
        feed(&mut r, b"start\nINFO all\n"),
        vec![
            Err(LinkError::UnknownCommand),
            Err(LinkError::UnknownCommand)
        ]
    );
}

#[test]
fn line_reader_accepts_32_bytes_and_rejects_33_or_more_as_too_long() {
    let mut r = LineReader::new();
    let mut input = vec![b'X'; 32];
    input.push(b'\n');
    input.extend([b'X'; 33]);
    input.push(b'\n');
    input.extend(b"START\n");
    assert_eq!(
        feed(&mut r, &input),
        vec![
            Err(LinkError::UnknownCommand),
            Err(LinkError::LineTooLong),
            Ok(Command::Start)
        ]
    );
}

#[test]
fn line_reader_joins_a_line_delivered_in_pieces() {
    let mut r = LineReader::new();
    assert_eq!(feed(&mut r, b"STA"), vec![]);
    assert_eq!(feed(&mut r, b"RT\n"), vec![Ok(Command::Start)]);
}

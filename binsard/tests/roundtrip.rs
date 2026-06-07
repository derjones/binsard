use binsard::{Decode, Encode};

#[derive(Encode, Decode, Debug, PartialEq)]
struct TestaVec {
    a: i32,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Testa {
    a: i32,
    b: u8,
    c: Option<u16>,
    d: Option<Vec<TestaVec>>,
    e: bool,
    g: [u8; 4],
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Start {
    a: i32,
    b: Option<i32>,
    c: TestEnum,
}

#[derive(Encode, Decode, Debug, PartialEq)]
enum TestEnum {
    A(Testa),
    B(Testa),
    C(Testa),
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct TupleWrapper(Vec<u8>);

#[derive(Encode, Decode, Debug, PartialEq)]
struct TuplePair(u32, u8);

#[derive(Encode, Decode, Debug, PartialEq)]
struct Marker;

#[derive(Encode, Decode, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Encode, Decode, Debug, PartialEq)]
enum Command {
    Ping,
    Move { x: i32, y: i32 },
    Say(TupleWrapper),
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct SensorReading {
    #[binsard(bits = 4)]
    channel: u8,
    #[binsard(bits = 12)]
    value: u16,
    active: bool,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct CompactFlags {
    flag_a: bool,
    flag_b: bool,
    flag_c: bool,
    #[binsard(bits = 3)]
    priority: u8,
    #[binsard(bits = 5)]
    seq_num: u8,
    payload: Vec<u8>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 2)]
enum SmallEnum {
    Off,
    Low,
    High,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 4)]
enum SensorMessage {
    Reset,
    Reading(SensorReading),
    Batch { count: u8, active: bool },
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Packet {
    #[binsard(bits = 4)]
    version: u8,
    mode: SmallEnum,
    msg: SensorMessage,
    #[binsard(bits = 13)]
    tracker_version: Option<u16>,
}

fn roundtrip<T: Encode + Decode + PartialEq + std::fmt::Debug>(value: &T) {
    let encoded = value.encode();
    let decoded = T::decode(&encoded).unwrap();
    assert_eq!(value, &decoded);
}

#[test]
fn struct_named_fields() {
    roundtrip(&Testa {
        a: 1,
        b: 2,
        c: Some(3),
        d: Some(vec![TestaVec { a: 4 }]),
        e: true,
        g: [1, 2, 3, 4],
    });
}

#[test]
fn struct_named_fields_none_options() {
    roundtrip(&Testa {
        a: 0,
        b: 0,
        c: None,
        d: None,
        e: false,
        g: [0, 0, 0, 0],
    });
}

#[test]
fn nested_struct_with_enum() {
    roundtrip(&Start {
        a: 1,
        b: Some(2),
        c: TestEnum::A(Testa {
            a: 1,
            b: 2,
            c: Some(3),
            d: Some(vec![TestaVec { a: 4 }]),
            e: true,
            g: [1, 2, 3, 4],
        }),
    });
}

#[test]
fn tuple_struct_vec() {
    roundtrip(&TupleWrapper(vec![10, 20, 30]));
}

#[test]
fn tuple_struct_pair() {
    roundtrip(&TuplePair(12345, 42));
}

#[test]
fn unit_struct() {
    roundtrip(&Marker);
}

#[test]
fn unit_enum_variants() {
    roundtrip(&Color::Red);
    roundtrip(&Color::Green);
    roundtrip(&Color::Blue);
}

#[test]
fn enum_named_fields() {
    roundtrip(&Command::Move { x: -10, y: 42 });
}

#[test]
fn enum_unit_variant() {
    roundtrip(&Command::Ping);
}

#[test]
fn enum_tuple_payload() {
    roundtrip(&Command::Say(TupleWrapper(vec![1, 2, 3])));
}

#[test]
fn bit_packed_struct() {
    roundtrip(&SensorReading {
        channel: 9,
        value: 3000,
        active: true,
    });
    let encoded = SensorReading {
        channel: 9,
        value: 3000,
        active: true,
    }
    .encode();
    assert!(encoded.len() <= 3, "17 bits should fit in 3 bytes");
}

#[test]
fn bit_packed_struct_max_values() {
    roundtrip(&SensorReading {
        channel: 15,
        value: 4095,
        active: false,
    });
}

#[test]
fn compact_flags() {
    roundtrip(&CompactFlags {
        flag_a: true,
        flag_b: false,
        flag_c: true,
        priority: 7,
        seq_num: 31,
        payload: vec![0xAA, 0xBB],
    });
}

#[test]
fn small_enum_2bit_tag() {
    for val in [SmallEnum::Off, SmallEnum::Low, SmallEnum::High] {
        let encoded = val.encode();
        assert_eq!(encoded.len(), 1, "2-bit tag fits in 1 byte");
        roundtrip(&val);
    }
}

#[test]
fn sensor_message_reset() {
    let msg = SensorMessage::Reset;
    let encoded = msg.encode();
    assert_eq!(encoded.len(), 1);
    roundtrip(&msg);
}

#[test]
fn sensor_message_reading() {
    roundtrip(&SensorMessage::Reading(SensorReading {
        channel: 5,
        value: 1234,
        active: true,
    }));
}

#[test]
fn sensor_message_batch() {
    roundtrip(&SensorMessage::Batch {
        count: 42,
        active: true,
    });
}

#[test]
fn packet_nested_bit_packed() {
    roundtrip(&Packet {
        version: 3,
        mode: SmallEnum::High,
        msg: SensorMessage::Reading(SensorReading {
            channel: 12,
            value: 2048,
            active: false,
        }),
        tracker_version: Some(8191),
    });
}

#[test]
fn packet_tracker_version_none() {
    roundtrip(&Packet {
        version: 1,
        mode: SmallEnum::Off,
        msg: SensorMessage::Reset,
        tracker_version: None,
    });
}

#[derive(Encode, Decode, Debug, PartialEq)]
enum BitPackedVariant {
    Simple,
    Packed {
        #[binsard(bits = 5)]
        level: u8,
        #[binsard(bits = 10)]
        position: u16,
        active: bool,
    },
}

#[test]
fn enum_variant_field_bits_attribute() {
    roundtrip(&BitPackedVariant::Simple);
    roundtrip(&BitPackedVariant::Packed {
        level: 31,
        position: 1023,
        active: true,
    });
    roundtrip(&BitPackedVariant::Packed {
        level: 0,
        position: 0,
        active: false,
    });
}

// --- String and f32/f64 roundtrips ---

#[derive(Encode, Decode, Debug, PartialEq)]
struct WithString {
    label: String,
    value: u32,
}

#[test]
fn string_ascii_roundtrip() {
    roundtrip(&WithString {
        label: "hello".into(),
        value: 42,
    });
}

#[test]
fn string_unicode_roundtrip() {
    roundtrip(&WithString {
        label: "Grüße 🌍".into(),
        value: 99,
    });
}

#[test]
fn string_empty_roundtrip() {
    roundtrip(&WithString {
        label: String::new(),
        value: 0,
    });
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct WithFloats {
    x: f32,
    y: f64,
}

#[test]
fn f32_f64_roundtrip() {
    roundtrip(&WithFloats { x: 1.5_f32, y: 123.456 });
    roundtrip(&WithFloats { x: 0.0, y: 0.0 });
    roundtrip(&WithFloats { x: f32::MAX, y: f64::MIN });
    roundtrip(&WithFloats { x: -1.0, y: f64::INFINITY });
}

// --- Empty Vec and edge cases ---

#[derive(Encode, Decode, Debug, PartialEq)]
struct WithOptionalVec {
    items: Option<Vec<TestaVec>>,
}

#[test]
fn empty_vec_roundtrip() {
    roundtrip(&WithOptionalVec {
        items: Some(vec![]),
    });
}

#[test]
fn none_optional_vec_roundtrip() {
    roundtrip(&WithOptionalVec { items: None });
}

// --- Extreme integer values ---

#[derive(Encode, Decode, Debug, PartialEq)]
struct ExtremeInts {
    a: i32,
    b: i64,
    c: u64,
    d: i16,
}

#[test]
fn extreme_integer_values() {
    roundtrip(&ExtremeInts {
        a: i32::MIN,
        b: i64::MIN,
        c: u64::MAX,
        d: i16::MAX,
    });
    roundtrip(&ExtremeInts {
        a: i32::MAX,
        b: i64::MAX,
        c: 0,
        d: i16::MIN,
    });
}

// --- Error case tests ---

#[test]
fn decode_truncated_struct_returns_error() {
    let result = TuplePair::decode(&[0x00]);
    assert_eq!(result, Err(binsard::DecodeError::UnexpectedEof));
}

#[test]
fn decode_empty_input_returns_error() {
    let result = TuplePair::decode(&[]);
    assert_eq!(result, Err(binsard::DecodeError::UnexpectedEof));
}

#[test]
fn decode_truncated_enum_returns_error() {
    let result = Color::decode(&[]);
    assert_eq!(result, Err(binsard::DecodeError::UnexpectedEof));
}

#[test]
fn decode_unknown_enum_tag_returns_error() {
    let mut enc = binsard::EncodeHelper::default();
    enc.write_partly(5u64, 8);
    let data = enc.finish();
    let result = Color::decode(&data);
    assert_eq!(result, Err(binsard::DecodeError::UnknownTag(5)));
}

#[test]
fn decode_truncated_string_returns_error() {
    let mut data = vec![10u8]; // string length=10 but no string data
    data.extend_from_slice(&42u32.to_be_bytes());
    let result = WithString::decode(&data);
    assert!(result.is_err());
}

#[test]
fn decode_truncated_vec_u8_returns_error() {
    let data = vec![0x00, 0x0A]; // Vec<u8> len=10 but no data
    let result = TupleWrapper::decode(&data);
    assert_eq!(result, Err(binsard::DecodeError::UnexpectedEof));
}

// --- Enum variants with shared field names (Bug 1 regression test) ---

#[derive(Encode, Decode, Debug, PartialEq)]
enum SharedFieldNames {
    Variant1 {
        #[binsard(bits = 4)]
        value: u8,
    },
    Variant2 {
        #[binsard(bits = 8)]
        value: u8,
    },
}

#[test]
fn enum_shared_field_names_different_bits() {
    roundtrip(&SharedFieldNames::Variant1 { value: 15 });
    roundtrip(&SharedFieldNames::Variant2 { value: 255 });

    // Variant1 uses 4 bits so value must be in [0, 15]
    let v1_max = SharedFieldNames::Variant1 { value: 15 };
    let decoded = SharedFieldNames::decode(&v1_max.encode()).unwrap();
    assert_eq!(decoded, v1_max);

    // Variant2 uses 8 bits so value can go up to 255
    let v2_max = SharedFieldNames::Variant2 { value: 255 };
    let decoded = SharedFieldNames::decode(&v2_max.encode()).unwrap();
    assert_eq!(decoded, v2_max);
}

// --- len_bytes attribute tests ---

#[derive(Encode, Decode, Debug, PartialEq)]
struct StringLenBytes2 {
    #[binsard(len_bytes = 2)]
    name: String,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct StringLenBytes4 {
    #[binsard(len_bytes = 4)]
    name: String,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct VecU8LenBytes1 {
    #[binsard(len_bytes = 1)]
    data: Vec<u8>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct VecU8LenBytes4 {
    #[binsard(len_bytes = 4)]
    data: Vec<u8>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct VecStructLenBytes2 {
    #[binsard(len_bytes = 2)]
    items: Vec<TestaVec>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct VecStructLenBytes4 {
    #[binsard(len_bytes = 4)]
    items: Vec<TestaVec>,
}

#[test]
fn string_len_bytes_2_roundtrip() {
    roundtrip(&StringLenBytes2 { name: "hello".into() });
    roundtrip(&StringLenBytes2 { name: String::new() });
    roundtrip(&StringLenBytes2 { name: "Grüße 🌍".into() });
    let encoded = StringLenBytes2 { name: "hi".into() }.encode();
    assert_eq!(encoded[0], 0, "u16 big-endian high byte");
    assert_eq!(encoded[1], 2, "u16 big-endian low byte = length 2");
}

#[test]
fn string_len_bytes_4_roundtrip() {
    roundtrip(&StringLenBytes4 { name: "test".into() });
    roundtrip(&StringLenBytes4 { name: String::new() });
    let encoded = StringLenBytes4 { name: "ab".into() }.encode();
    assert_eq!(&encoded[0..4], &[0, 0, 0, 2], "u32 big-endian length = 2");
}

#[test]
fn vec_u8_len_bytes_1_roundtrip() {
    roundtrip(&VecU8LenBytes1 { data: vec![1, 2, 3] });
    roundtrip(&VecU8LenBytes1 { data: vec![] });
    let encoded = VecU8LenBytes1 { data: vec![0xAA, 0xBB] }.encode();
    assert_eq!(encoded[0], 2, "u8 length prefix = 2");
    assert_eq!(encoded.len(), 3);
}

#[test]
fn vec_u8_len_bytes_4_roundtrip() {
    roundtrip(&VecU8LenBytes4 { data: vec![10, 20, 30] });
    roundtrip(&VecU8LenBytes4 { data: vec![] });
    let encoded = VecU8LenBytes4 { data: vec![0xFF] }.encode();
    assert_eq!(&encoded[0..4], &[0, 0, 0, 1], "u32 big-endian length = 1");
    assert_eq!(encoded.len(), 5);
}

#[test]
fn vec_struct_len_bytes_2_roundtrip() {
    roundtrip(&VecStructLenBytes2 {
        items: vec![TestaVec { a: 1 }, TestaVec { a: 2 }],
    });
    roundtrip(&VecStructLenBytes2 { items: vec![] });
    let encoded = VecStructLenBytes2 {
        items: vec![TestaVec { a: 42 }],
    }.encode();
    assert_eq!(encoded[0], 0, "u16 big-endian high byte");
    assert_eq!(encoded[1], 1, "u16 big-endian low byte = count 1");
}

#[test]
fn vec_struct_len_bytes_4_roundtrip() {
    roundtrip(&VecStructLenBytes4 {
        items: vec![TestaVec { a: 100 }, TestaVec { a: 200 }],
    });
    roundtrip(&VecStructLenBytes4 { items: vec![] });
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct MixedLenBytes {
    #[binsard(len_bytes = 4)]
    label: String,
    #[binsard(len_bytes = 1)]
    payload: Vec<u8>,
    #[binsard(len_bytes = 2)]
    items: Vec<TestaVec>,
    value: u32,
}

#[test]
fn mixed_len_bytes_roundtrip() {
    roundtrip(&MixedLenBytes {
        label: "mixed test".into(),
        payload: vec![1, 2, 3, 4, 5],
        items: vec![TestaVec { a: 42 }, TestaVec { a: -1 }],
        value: 999,
    });
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct DefaultLenBytes {
    name: String,
    data: Vec<u8>,
}

#[test]
fn default_len_bytes_unchanged() {
    roundtrip(&DefaultLenBytes {
        name: "default".into(),
        data: vec![1, 2, 3],
    });
    let encoded = DefaultLenBytes {
        name: "hi".into(),
        data: vec![0xAA],
    }.encode();
    assert_eq!(encoded[0], 2, "String default: u8 length prefix");
    assert_eq!(encoded[3], 0, "Vec<u8> default: u16 high byte");
    assert_eq!(encoded[4], 1, "Vec<u8> default: u16 low byte");
}

// --- Deep complex mixed struct/enum test ---

#[derive(Encode, Decode, Debug, PartialEq)]
struct GeoCoord {
    #[binsard(bits = 1)]
    hemisphere: u8,
    #[binsard(bits = 9)]
    degrees: u16,
    #[binsard(bits = 6)]
    minutes: u8,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Location {
    lat: GeoCoord,
    lon: GeoCoord,
    #[binsard(len_bytes = 2)]
    name: String,
    altitude: Option<f32>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 3)]
enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 2)]
enum CompressionKind {
    None,
    Gzip,
    Zstd,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct BlobHeader {
    #[binsard(bits = 4)]
    version: u8,
    compression: CompressionKind,
    encrypted: bool,
    #[binsard(len_bytes = 4)]
    data: Vec<u8>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct LogEntry {
    timestamp: u64,
    severity: Severity,
    #[binsard(len_bytes = 4)]
    message: String,
    #[binsard(len_bytes = 2)]
    tags: Vec<TagKV>,
    source: Option<Location>,
    attachment: Option<BlobHeader>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct TagKV {
    #[binsard(len_bytes = 1)]
    key: String,
    #[binsard(len_bytes = 1)]
    value: String,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 4)]
enum TransportPayload {
    Empty,
    SingleLog(LogEntry),
    Batch {
        #[binsard(bits = 10)]
        sequence: u16,
        #[binsard(len_bytes = 4)]
        entries: Vec<LogEntry>,
    },
    RawDump {
        #[binsard(len_bytes = 4)]
        label: String,
        blob: BlobHeader,
        checksum: [u8; 4],
    },
    Heartbeat {
        #[binsard(bits = 16)]
        uptime_secs: u16,
        healthy: bool,
    },
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct TransportFrame {
    #[binsard(bits = 4)]
    proto_version: u8,
    #[binsard(bits = 12)]
    frame_id: u16,
    #[binsard(bits = 6)]
    flags: u8,
    urgent: bool,
    payload: TransportPayload,
    #[binsard(len_bytes = 1)]
    trailing_metadata: Vec<u8>,
}

fn make_location(name: &str) -> Location {
    Location {
        lat: GeoCoord { hemisphere: 0, degrees: 48, minutes: 51 },
        lon: GeoCoord { hemisphere: 1, degrees: 2, minutes: 21 },
        name: name.into(),
        altitude: Some(35.5),
    }
}

fn make_blob(data: &[u8], compressed: bool) -> BlobHeader {
    BlobHeader {
        version: 3,
        compression: if compressed { CompressionKind::Zstd } else { CompressionKind::None },
        encrypted: true,
        data: data.to_vec(),
    }
}

fn make_log_entry(sev: Severity, msg: &str, with_loc: bool, with_blob: bool) -> LogEntry {
    LogEntry {
        timestamp: 1_700_000_000_000,
        severity: sev,
        message: msg.into(),
        tags: vec![
            TagKV { key: "env".into(), value: "prod".into() },
            TagKV { key: "region".into(), value: "eu-west-1".into() },
        ],
        source: if with_loc { Some(make_location("Datacenter-7")) } else { None },
        attachment: if with_blob { Some(make_blob(&[0xDE, 0xAD, 0xBE, 0xEF], true)) } else { None },
    }
}

#[test]
fn deep_complex_empty_payload() {
    roundtrip(&TransportFrame {
        proto_version: 7,
        frame_id: 4095,
        flags: 0b101010,
        urgent: false,
        payload: TransportPayload::Empty,
        trailing_metadata: vec![],
    });
}

#[test]
fn deep_complex_heartbeat() {
    roundtrip(&TransportFrame {
        proto_version: 1,
        frame_id: 1,
        flags: 0,
        urgent: true,
        payload: TransportPayload::Heartbeat {
            uptime_secs: 65535,
            healthy: true,
        },
        trailing_metadata: vec![0xFF],
    });
}

#[test]
fn deep_complex_single_log_full() {
    roundtrip(&TransportFrame {
        proto_version: 2,
        frame_id: 100,
        flags: 0b111111,
        urgent: true,
        payload: TransportPayload::SingleLog(
            make_log_entry(Severity::Error, "disk failure on /dev/sda1", true, true),
        ),
        trailing_metadata: vec![0x01, 0x02, 0x03],
    });
}

#[test]
fn deep_complex_single_log_minimal() {
    roundtrip(&TransportFrame {
        proto_version: 0,
        frame_id: 0,
        flags: 0,
        urgent: false,
        payload: TransportPayload::SingleLog(LogEntry {
            timestamp: 0,
            severity: Severity::Trace,
            message: String::new(),
            tags: vec![],
            source: None,
            attachment: None,
        }),
        trailing_metadata: vec![],
    });
}

#[test]
fn deep_complex_batch() {
    roundtrip(&TransportFrame {
        proto_version: 15,
        frame_id: 2048,
        flags: 0b010101,
        urgent: false,
        payload: TransportPayload::Batch {
            sequence: 1023,
            entries: vec![
                make_log_entry(Severity::Info, "startup complete", false, false),
                make_log_entry(Severity::Warn, "memory pressure 🔥", true, false),
                make_log_entry(Severity::Fatal, "kernel panic", true, true),
            ],
        },
        trailing_metadata: vec![0xCA, 0xFE],
    });
}

#[test]
fn deep_complex_raw_dump() {
    roundtrip(&TransportFrame {
        proto_version: 5,
        frame_id: 3333,
        flags: 0b000001,
        urgent: true,
        payload: TransportPayload::RawDump {
            label: "core-dump-2026-03-04T12:00:00Z".into(),
            blob: make_blob(&vec![0xAB; 256], false),
            checksum: [0xDE, 0xAD, 0xBE, 0xEF],
        },
        trailing_metadata: vec![],
    });
}

#[test]
fn deep_complex_batch_empty_entries() {
    roundtrip(&TransportFrame {
        proto_version: 8,
        frame_id: 999,
        flags: 0b110011,
        urgent: false,
        payload: TransportPayload::Batch {
            sequence: 0,
            entries: vec![],
        },
        trailing_metadata: vec![0x00],
    });
}

#[test]
fn deep_complex_unicode_everywhere() {
    roundtrip(&TransportFrame {
        proto_version: 1,
        frame_id: 42,
        flags: 0,
        urgent: false,
        payload: TransportPayload::SingleLog(LogEntry {
            timestamp: 999_999_999_999,
            severity: Severity::Debug,
            message: "Ünïcödé tëst 日本語 🚀🔥".into(),
            tags: vec![
                TagKV { key: "名前".into(), value: "テスト".into() },
                TagKV { key: "données".into(), value: "résultat".into() },
            ],
            source: Some(Location {
                lat: GeoCoord { hemisphere: 1, degrees: 139, minutes: 41 },
                lon: GeoCoord { hemisphere: 0, degrees: 35, minutes: 40 },
                name: "東京タワー".into(),
                altitude: None,
            }),
            attachment: Some(BlobHeader {
                version: 15,
                compression: CompressionKind::Gzip,
                encrypted: false,
                data: vec![0xFF; 100],
            }),
        }),
        trailing_metadata: vec![0x42, 0x43, 0x44, 0x45, 0x46],
    });
}

// --- Regression: [u8; N] in enum named fields ---

#[derive(Encode, Decode, Debug, PartialEq)]
enum EnumWithArrayField {
    Empty,
    WithArray {
        tag: u8,
        hash: [u8; 8],
        suffix: u16,
    },
    MixedArray {
        flag: bool,
        digest: [u8; 32],
        #[binsard(bits = 4)]
        priority: u8,
    },
}

#[test]
fn enum_named_field_array_roundtrip() {
    roundtrip(&EnumWithArrayField::Empty);
    roundtrip(&EnumWithArrayField::WithArray {
        tag: 0xFF,
        hash: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        suffix: 9999,
    });
    roundtrip(&EnumWithArrayField::WithArray {
        tag: 0,
        hash: [0; 8],
        suffix: 0,
    });
    roundtrip(&EnumWithArrayField::MixedArray {
        flag: true,
        digest: [0xAB; 32],
        priority: 15,
    });
    roundtrip(&EnumWithArrayField::MixedArray {
        flag: false,
        digest: [0; 32],
        priority: 0,
    });
}

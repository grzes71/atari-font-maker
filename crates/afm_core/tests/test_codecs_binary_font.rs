use std::fs;
use std::io::Cursor;
use std::path::Path;

use afm_core::codecs::binary_fnt::{self, DUAL_FONT_SIZE};
use afm_core::constants::FONT_BANK_SIZE;
use afm_core::error::FontFormatError;
use afm_core::font::bank::FontBankSet;

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

#[test]
fn test_fnt_loading_and_saving_parity_golden() {
    let default_fnt_bytes =
        fs::read(fixture_path("projects/Default.fnt")).expect("Read Default.fnt");
    assert_eq!(default_fnt_bytes.len(), FONT_BANK_SIZE);

    // 1. Direct codec load
    let mut cursor = Cursor::new(&default_fnt_bytes);
    let loaded_bytes = binary_fnt::load_fnt(&mut cursor).expect("Direct load_fnt");
    assert_eq!(loaded_bytes.as_slice(), default_fnt_bytes.as_slice());

    // 2. Direct codec save
    let mut out_buf = Vec::new();
    binary_fnt::save_fnt(&loaded_bytes, &mut out_buf).expect("Direct save_fnt");
    assert_eq!(out_buf, default_fnt_bytes);

    // 3. FontBankSet integration across all 4 banks
    for bank_idx in 0..4 {
        let mut bank_set = FontBankSet::new();
        let mut cur = Cursor::new(&default_fnt_bytes);
        bank_set
            .load_fnt(bank_idx, &mut cur)
            .expect("load_fnt into bank");

        // Verify other banks remain empty
        for other_bank in 0..4 {
            let start = other_bank * FONT_BANK_SIZE;
            let slice = &bank_set.as_bytes()[start..start + FONT_BANK_SIZE];
            if other_bank == bank_idx {
                assert_eq!(slice, default_fnt_bytes.as_slice());
            } else {
                assert_eq!(slice, [0u8; FONT_BANK_SIZE].as_slice());
            }
        }

        // Verify save_fnt from bank
        let mut saved_bank = Vec::new();
        bank_set
            .save_fnt(bank_idx, &mut saved_bank)
            .expect("save_fnt from bank");
        assert_eq!(saved_bank, default_fnt_bytes);
    }
}

#[test]
fn test_fn2_loading_and_saving_parity_golden() {
    let dual_fn2_bytes =
        fs::read(fixture_path("projects/dual_sample.fn2")).expect("Read dual_sample.fn2");
    assert_eq!(dual_fn2_bytes.len(), DUAL_FONT_SIZE);

    // 1. Direct codec load
    let mut cursor = Cursor::new(&dual_fn2_bytes);
    let loaded_bytes = binary_fnt::load_fn2(&mut cursor).expect("Direct load_fn2");
    assert_eq!(loaded_bytes.as_slice(), dual_fn2_bytes.as_slice());

    // 2. Direct codec save
    let mut out_buf = Vec::new();
    binary_fnt::save_fn2(&loaded_bytes, &mut out_buf).expect("Direct save_fn2");
    assert_eq!(out_buf, dual_fn2_bytes);

    // 3. FontBankSet integration across start banks 0, 1, 2
    for start_bank in 0..=2 {
        let mut bank_set = FontBankSet::new();
        let mut cur = Cursor::new(&dual_fn2_bytes);
        bank_set
            .load_fn2(start_bank, &mut cur)
            .expect("load_fn2 into bank");

        // Verify loaded banks
        let start = start_bank * FONT_BANK_SIZE;
        assert_eq!(
            &bank_set.as_bytes()[start..start + DUAL_FONT_SIZE],
            dual_fn2_bytes.as_slice()
        );

        // Verify save_fn2
        let mut saved_dual = Vec::new();
        bank_set
            .save_fn2(start_bank, &mut saved_dual)
            .expect("save_fn2 from bank");
        assert_eq!(saved_dual, dual_fn2_bytes);
    }
}

#[test]
fn test_fnt_roundtrip_synthetic() {
    let mut synthetic_data = [0u8; FONT_BANK_SIZE];
    for (i, byte) in synthetic_data.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }

    let mut out_bytes = Vec::new();
    binary_fnt::save_fnt(&synthetic_data, &mut out_bytes).unwrap();
    assert_eq!(out_bytes.len(), FONT_BANK_SIZE);

    let mut cur = Cursor::new(&out_bytes);
    let loaded = binary_fnt::load_fnt(&mut cur).unwrap();
    assert_eq!(loaded, synthetic_data);
}

#[test]
fn test_fn2_roundtrip_synthetic() {
    let mut synthetic_data = [0u8; DUAL_FONT_SIZE];
    for (i, byte) in synthetic_data.iter_mut().enumerate() {
        *byte = ((i * 7 + 13) % 256) as u8;
    }

    let mut out_bytes = Vec::new();
    binary_fnt::save_fn2(&synthetic_data, &mut out_bytes).unwrap();
    assert_eq!(out_bytes.len(), DUAL_FONT_SIZE);

    let mut cur = Cursor::new(&out_bytes);
    let loaded = binary_fnt::load_fn2(&mut cur).unwrap();
    assert_eq!(loaded, synthetic_data);
}

#[test]
fn test_fnt_malformed_and_truncated_inputs() {
    let test_sizes = [0, 1, 10, 512, 1023, 1025, 2048];
    for size in test_sizes {
        let bad_data = vec![0xAAu8; size];
        let mut cur = Cursor::new(&bad_data);
        let res = binary_fnt::load_fnt(&mut cur);
        match res {
            Err(FontFormatError::InvalidSize { expected, actual }) => {
                assert_eq!(expected, FONT_BANK_SIZE);
                assert_eq!(actual, size);
            }
            other => panic!(
                "Expected InvalidSize error for size {}, got {:?}",
                size, other
            ),
        }
    }
}

#[test]
fn test_fn2_malformed_and_truncated_inputs() {
    let test_sizes = [0, 1, 512, 1024, 2047, 2049, 4096];
    for size in test_sizes {
        let bad_data = vec![0x55u8; size];
        let mut cur = Cursor::new(&bad_data);
        let res = binary_fnt::load_fn2(&mut cur);
        match res {
            Err(FontFormatError::InvalidSize { expected, actual }) => {
                assert_eq!(expected, DUAL_FONT_SIZE);
                assert_eq!(actual, size);
            }
            other => panic!(
                "Expected InvalidSize error for size {}, got {:?}",
                size, other
            ),
        }
    }
}

#[test]
fn test_bank_boundary_validation() {
    let mut bank_set = FontBankSet::new();
    let default_fnt_bytes = fs::read(fixture_path("projects/Default.fnt")).unwrap();

    // Invalid bank for load_fnt / save_fnt (valid is 0..=3)
    let mut cur1 = Cursor::new(&default_fnt_bytes);
    assert!(matches!(
        bank_set.load_fnt(4, &mut cur1),
        Err(FontFormatError::InvalidBankIndex(4))
    ));
    assert!(matches!(
        bank_set.save_fnt(4, &mut Vec::new()),
        Err(FontFormatError::InvalidBankIndex(4))
    ));

    // Invalid start_bank for load_fn2 / save_fn2 (valid is 0..=2)
    let dual_fn2_bytes = fs::read(fixture_path("projects/dual_sample.fn2")).unwrap();
    let mut cur2 = Cursor::new(&dual_fn2_bytes);
    assert!(matches!(
        bank_set.load_fn2(3, &mut cur2),
        Err(FontFormatError::InvalidBankIndex(3))
    ));
    assert!(matches!(
        bank_set.save_fn2(3, &mut Vec::new()),
        Err(FontFormatError::InvalidBankIndex(3))
    ));
}

// Custom DES ECB decryption for Native32 encrypted headers.
// This is NOT a standard DES implementation - it uses the same algorithm
// as the reference implementation to ensure compatibility.

use crate::des_constants::*;

fn expand_bits(data: &[u8], count: usize) -> Vec<u8> {
    let mut result = vec![0u8; count];
    for i in 0..count {
        result[i] = (data[i >> 3] >> (i & 7)) & 1;
    }
    result
}

fn compress_bits(data: &[u8], count: usize) -> Vec<u8> {
    let mut result = vec![0u8; count / 8];
    for i in 0..count {
        result[i >> 3] |= data[i] << (i & 7);
    }
    result
}

fn do_shuffle(dst: &mut [u8], src: &[u8], table: &[u8], count: usize, offset: usize) {
    let mut temp = vec![0u8; count];
    for i in 0..count {
        temp[i] = src[(table[i] - 1) as usize];
    }
    for i in 0..count {
        dst[i + offset] = temp[i];
    }
}

fn slice_and_dice(src: &mut [u8], count: usize, splitpoint: usize, offset: usize) {
    let mut shuffle_temp = vec![0u8; splitpoint];
    shuffle_temp.copy_from_slice(&src[offset..offset + splitpoint]);
    for i in 0..(count - splitpoint) {
        src[offset + i] = src[offset + i + splitpoint];
    }
    for i in 0..splitpoint {
        src[offset + i + (count - splitpoint)] = shuffle_temp[i];
    }
}

fn expand_key(src: &[u8]) -> Vec<u8> {
    let key_bits_raw = expand_bits(src, 0x40);
    let mut key_bits = key_bits_raw;
    do_shuffle(
        &mut key_bits.clone(),
        &key_bits,
        &INITIAL_KEY_PERMUTATION,
        0x38,
        0,
    );
    // Re-do with proper in-place
    let mut kb = vec![0u8; 0x38];
    for i in 0..0x38 {
        kb[i] = key_bits[(INITIAL_KEY_PERMUTATION[i] - 1) as usize];
    }
    key_bits = kb;

    let mut result = vec![0u8; 0x30 * 0x10];
    for i in 0..0x10 {
        let splitpoint = KEY_SHIFT_SIZES[i] as usize;
        slice_and_dice(&mut key_bits, 0x1c, splitpoint, 0);
        slice_and_dice(&mut key_bits, 0x1c, splitpoint, 0x1c);
        do_shuffle(&mut result, &key_bits, &SUB_KEY_PERMUTATION, 0x30, i * 0x30);
    }
    result
}

fn do_sbox(data: &mut [u8], key: &[u8]) {
    for i in 0..8 {
        let k = &key[i * 6..(i + 1) * 6];
        let idx = (i * 4 + k[5] as usize + k[0] as usize * 2) * 0x10
            + (k[4] as usize + k[1] as usize * 8 + k[2] as usize * 4 + k[3] as usize * 2);
        let bits = expand_bits(&DES_SBOXES[idx..idx + 1], 4);
        for j in 0..4 {
            data[i * 4 + j] = bits[j];
        }
    }
}

fn process_iteration(data: &mut [u8], key: &[u8]) {
    let mut iter_temp = vec![0u8; 0x30];
    do_shuffle(&mut iter_temp, data, &MESSAGE_SHUFFLE, 0x30, 0);
    for i in 0..0x30 {
        iter_temp[i] ^= key[i];
    }
    do_sbox(data, &iter_temp);
    let data_copy = data.to_vec();
    do_shuffle(data, &data_copy, &RIGHT_SUB_MESSAGE_PERMUTATION, 0x20, 0);
}

fn decrypt_chunk(src: &[u8], expanded_key: &[u8]) -> Vec<u8> {
    let expanded_data_raw = expand_bits(src, 0x40);
    let mut expanded_data = vec![0u8; 0x40];
    for i in 0..0x40 {
        expanded_data[i] = expanded_data_raw[(INITIAL_MESSAGE_PERMUTATION[i] - 1) as usize];
    }

    let mut i: isize = 0x2d0;
    while i >= 0 {
        let idx = i as usize;
        let mut temp_data = [0u8; 0x20];
        temp_data.copy_from_slice(&expanded_data[..0x20]);
        process_iteration(&mut expanded_data, &expanded_key[idx..idx + 0x30]);
        for j in 0..0x20 {
            expanded_data[j] ^= expanded_data[0x20 + j];
        }
        for j in 0..0x20 {
            expanded_data[0x20 + j] = temp_data[j];
        }
        i -= 0x30;
    }

    let final_data = expanded_data.clone();
    for i in 0..0x40 {
        expanded_data[i] = final_data[(FINAL_MESSAGE_PERMUTATION[i] - 1) as usize];
    }
    compress_bits(&expanded_data, 0x40)
}

fn do_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    let expanded_key = expand_key(key);
    let mut result = Vec::new();
    for i in 0..(data.len() / 8) {
        result.extend_from_slice(&decrypt_chunk(&data[i * 8..(i + 1) * 8], &expanded_key));
    }
    result
}

/// Try all 5 candidate keys and return the decrypted header if the `8202` magic is found.
pub fn decrypt_header(data: &[u8]) -> Option<Vec<u8>> {
    let keys: &[&[u8]] = &[
        b"11111111",
        b"22222222",
        b"aaaaaaaa",
        b"bbbbbbbb",
        b"aber3801",
    ];
    for key in keys {
        let decrypted = do_decrypt(data, key);
        if decrypted.len() >= 8 && &decrypted[4..8] == b"8202" {
            log::info!(
                "Using DES key: {:?}",
                std::str::from_utf8(key).unwrap_or("?")
            );
            return Some(decrypted);
        }
    }
    None
}

// src/helpers/code_generator.rs

use rand::{thread_rng, Rng};

// Un jeu de caractères facile à lire, sans 'O', '0', 'I', '1' pour éviter la confusion.
const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_LENGTH: usize = 9;
const CHUNK_SIZE: usize = 3;

/// Génère un code de liaison unique et formaté (ex: "ABC-DEF-GHI").
pub fn generer_code_unique() -> String {
    let mut rng = thread_rng();
    let mut result = String::with_capacity(CODE_LENGTH + 2); // 9 chars + 2 tirets

    for i in 0..CODE_LENGTH {
        // Ajoute un tiret tous les 3 caractères (sauf au début)
        if i > 0 && i % CHUNK_SIZE == 0 {
            result.push('-');
        }

        // Choisit un caractère aléatoire dans notre jeu de caractères
        let random_char = CHARSET[rng.gen_range(0..CHARSET.len())] as char;
        result.push(random_char);
    }

    result
}
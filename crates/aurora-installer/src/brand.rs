//! A cara da AURORA no terminal: as cores do tema Boreal e o camaleão desenhado
//! em braille, do mesmo contorno do `assets/brand/aurora-mark.svg`.

use std::sync::OnceLock;

use ratatui::style::Color;

/// O contorno do camaleão, o mesmo do `aurora-mark.svg` e do `AuroraMark` da
/// IDE, no quadro de [`VIEW_BOX`].
const MARK: &str = "M3930 1801l0 282l57 -7c768 -88 1469 233 1886 864c544 825 369 1947 -402 2566c-359 289 -769 434 -1223 434c-312 0 -590 -66 -873 -208c-161 -80 -371 -226 -452 -314l-22 -24l173 -173l174 -174l74 60c240 197 496 303 806 333c145 15 316 0 479 -40c563 -138 914 -496 1050 -1070c30 -130 42 -390 24 -521c-80 -553 -453 -1004 -978 -1178c-165 -55 -239 -66 -458 -66c-213 0 -264 7 -440 62c-83 26 -249 102 -323 148l-42 26l0 -315c0 -208 -3 -316 -10 -316c-21 0 -187 89 -275 147c-301 199 -532 454 -691 765l-55 108l48 0c26 0 430 -5 897 -10c888 -11 940 -10 1078 31c437 128 726 557 678 1009c-12 118 -38 210 -90 318c-50 104 -86 157 -169 244c-201 214 -502 324 -793 290c-179 -21 -380 -106 -495 -208l-48 -43l174 -174l174 -175l49 34c94 64 240 90 357 63c195 -45 350 -238 351 -436c0 -203 -128 -381 -319 -443l-70 -22l-1231 8l-1230 9l0 -31c0 -63 53 -294 101 -439c144 -433 412 -832 753 -1119c276 -233 589 -402 928 -501c118 -34 309 -75 354 -75l24 0l0 281z";

/// O quadro do contorno: x, y, largura e altura, como no `viewBox` do SVG.
const VIEW_BOX: (f64, f64, f64, f64) = (1769.4, 1520.0, 4420.0, 4420.0);

/// Quanto de cada ponto o contorno precisa cobrir para o ponto acender:
/// abaixo da metade, para as partes finas da espiral não se partirem.
const COVERAGE: f64 = 0.4;

/// O terminal mostra cor de 24 bits? (`COLORTERM=truecolor` ou `24bit`.)
/// Sem isso, as cores da marca vão para as mais próximas da paleta de 256.
fn truecolor() -> bool {
    static TRUECOLOR: OnceLock<bool> = OnceLock::new();
    *TRUECOLOR.get_or_init(|| {
        std::env::var("COLORTERM")
            .map(|v| matches!(v.as_str(), "truecolor" | "24bit"))
            .unwrap_or(false)
    })
}

/// O azul do tema Boreal no escuro (`#59a0f9`, o do camaleão): títulos, o item
/// escolhido, a barra de progresso.
pub fn accent() -> Color {
    if truecolor() {
        Color::Rgb(0x59, 0xa0, 0xf9)
    } else {
        Color::Indexed(75)
    }
}

/// O texto claro do tema (`#eaedf1`): o nome da AURORA ao lado da marca.
pub fn light() -> Color {
    if truecolor() {
        Color::Rgb(0xea, 0xed, 0xf1)
    } else {
        Color::Indexed(255)
    }
}

/// A marca em braille, com `cols` colunas de largura. Cada caractere tem 2
/// por 4 pontos e a célula do terminal tem o dobro da altura, então o
/// quadrado do camaleão ocupa `cols / 2` linhas.
pub fn mark(cols: usize) -> Vec<String> {
    const SAMPLES: usize = 4;
    let width = cols * 2;
    let rows = width.div_ceil(4);
    let height = rows * 4;
    let (x0, y0, w, _) = VIEW_BOX;
    let scale = w / width as f64;
    // Com `cols` ímpar sobra uma faixa de pontos: o desenho fica no meio dela.
    let top = y0 - (height - width) as f64 / 2.0 * scale;
    let edges = edges(MARK);
    let sample = |i: usize, s: usize| (i as f64 + (s as f64 + 0.5) / SAMPLES as f64) * scale;
    let mut dots = vec![vec![false; width]; height];
    for (row, line) in dots.iter_mut().enumerate() {
        for (col, dot) in line.iter_mut().enumerate() {
            let hits = (0..SAMPLES * SAMPLES)
                .filter(|s| {
                    inside(
                        &edges,
                        x0 + sample(col, s % SAMPLES),
                        top + sample(row, s / SAMPLES),
                    )
                })
                .count();
            *dot = hits as f64 >= COVERAGE * (SAMPLES * SAMPLES) as f64;
        }
    }
    // Os pontos de cada caractere braille, na ordem dos bits do Unicode.
    const BITS: [(usize, usize, u32); 8] = [
        (0, 0, 0x01),
        (0, 1, 0x02),
        (0, 2, 0x04),
        (1, 0, 0x08),
        (1, 1, 0x10),
        (1, 2, 0x20),
        (0, 3, 0x40),
        (1, 3, 0x80),
    ];
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    let bits = BITS
                        .iter()
                        .filter(|(dx, dy, _)| dots[r * 4 + dy][c * 2 + dx])
                        .fold(0, |acc, (_, _, bit)| acc | bit);
                    char::from_u32(0x2800 + bits).unwrap_or(' ')
                })
                .collect::<String>()
                .trim_end_matches('\u{2800}')
                .to_owned()
        })
        .collect()
}

/// O ponto (`x`, `y`) está dentro do contorno, pela regra nonzero?
fn inside(edges: &[[f64; 4]], x: f64, y: f64) -> bool {
    let mut winding = 0;
    for &[ax, ay, bx, by] in edges {
        if (ay <= y && y < by) || (by <= y && y < ay) {
            let cross = ax + (y - ay) * (bx - ax) / (by - ay);
            if cross > x {
                winding += if by > ay { 1 } else { -1 };
            }
        }
    }
    winding != 0
}

/// As arestas não horizontais dos contornos de `path`.
fn edges(path: &str) -> Vec<[f64; 4]> {
    let mut out = Vec::new();
    for contour in contours(path) {
        let closing = contour.first().copied();
        for (a, b) in contour
            .iter()
            .zip(contour.iter().skip(1).chain(closing.as_ref()))
        {
            if a.1 != b.1 {
                out.push([a.0, a.1, b.0, b.1]);
            }
        }
    }
    out
}

/// Os contornos de um caminho SVG em polígonos, cada curva em 12 retas. Lê
/// só o que o contorno da marca usa: `M`, `L`, `C` e `Z`, absolutos ou
/// relativos, com os números separados por espaço.
fn contours(path: &str) -> Vec<Vec<(f64, f64)>> {
    const STEPS: usize = 12;
    let spaced: String = path
        .chars()
        .flat_map(|c| {
            if c.is_ascii_alphabetic() {
                vec![' ', c, ' ']
            } else {
                vec![c]
            }
        })
        .collect();
    let mut tokens = spaced.split_whitespace().peekable();
    let mut shapes = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let (mut x, mut y, mut start) = (0.0, 0.0, (0.0, 0.0));
    let mut command = 'M';
    while let Some(&token) = tokens.peek() {
        if token.chars().all(|c| c.is_ascii_alphabetic()) {
            command = token.chars().next().unwrap_or('M');
            tokens.next();
            if command.eq_ignore_ascii_case(&'z') {
                if !current.is_empty() {
                    shapes.push(std::mem::take(&mut current));
                }
                (x, y) = start;
            }
            continue;
        }
        let mut number = || {
            tokens
                .next()
                .and_then(|t| t.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let (ox, oy) = if command.is_ascii_lowercase() {
            (x, y)
        } else {
            (0.0, 0.0)
        };
        match command.to_ascii_uppercase() {
            'M' => {
                if !current.is_empty() {
                    shapes.push(std::mem::take(&mut current));
                }
                (x, y) = (ox + number(), oy + number());
                start = (x, y);
                current.push((x, y));
                // Pares depois do M são retas, como no SVG.
                command = if command.is_ascii_lowercase() {
                    'l'
                } else {
                    'L'
                };
            }
            'L' => {
                (x, y) = (ox + number(), oy + number());
                current.push((x, y));
            }
            'C' => {
                let c1 = (ox + number(), oy + number());
                let c2 = (ox + number(), oy + number());
                let end = (ox + number(), oy + number());
                for step in 1..=STEPS {
                    let t = step as f64 / STEPS as f64;
                    let u = 1.0 - t;
                    let at = |p0: f64, p1: f64, p2: f64, p3: f64| {
                        u * u * u * p0
                            + 3.0 * u * u * t * p1
                            + 3.0 * u * t * t * p2
                            + t * t * t * p3
                    };
                    current.push((at(x, c1.0, c2.0, end.0), at(y, c1.1, c2.1, end.1)));
                }
                (x, y) = end;
            }
            _ => {
                number();
            }
        }
    }
    if !current.is_empty() {
        shapes.push(current);
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_square_and_fills_its_frame() {
        let lines = mark(12);
        assert_eq!(lines.len(), 6, "{lines:#?}");
        assert!(lines.iter().all(|l| l.chars().count() <= 12), "{lines:#?}");
        // O camaleão encosta nas quatro bordas do quadro: tem ponto na
        // primeira e na última linha e na primeira e na última coluna.
        let blank = |c: char| c == '\u{2800}';
        assert!(
            lines.first().unwrap().chars().any(|c| !blank(c)),
            "{lines:#?}"
        );
        assert!(
            lines.last().unwrap().chars().any(|c| !blank(c)),
            "{lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.chars().next().is_some_and(|c| !blank(c))),
            "{lines:#?}"
        );
        assert!(lines.iter().any(|l| l.chars().count() == 12), "{lines:#?}");
    }

    #[test]
    fn the_outline_is_one_closed_contour_inside_the_frame() {
        let shapes = contours(MARK);
        assert_eq!(shapes.len(), 1);
        let (x0, y0, w, h) = VIEW_BOX;
        assert!(shapes[0].len() > 100);
        assert!(shapes[0].iter().all(|&(x, y)| {
            (x0 - 1.0..=x0 + w + 1.0).contains(&x) && (y0 - 1.0..=y0 + h + 1.0).contains(&y)
        }));
        // O caminho volta ao ponto em que começou.
        assert_eq!(shapes[0].first(), Some(&(3930.0, 1801.0)));
        assert_eq!(shapes[0].last(), Some(&(3930.0, 1801.0)));
    }

    #[test]
    fn relative_and_absolute_commands_give_the_same_points() {
        let relative = contours("M10 10l5 0c0 5 5 5 5 10z");
        let absolute = contours("M10 10L15 10C15 15 20 15 20 20Z");
        assert_eq!(relative, absolute);
        assert_eq!(relative[0].len(), 2 + 12);
    }
}

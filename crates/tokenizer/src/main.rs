use std::fs::read_to_string;
use std::time::Instant;
use tokenizer::Tokenizer;

fn main() {
  let file_list = [
    ("tailwind-components.css", "2.8K"),
    ("bootstrap-reboot.css", "7.4K"),
    ("bootstrap-grid.css", "71K"),
    ("bootstrap.css", "201K"),
    ("tailwind.css", "3.5M"),
    ("tailwind-dark.css", "5.8M"),
  ];

  for (file, size) in file_list {
    let css: &str = &read_to_string(format!("../../assets/{}", file)).unwrap();
    let start = Instant::now();
    let processor = Tokenizer::new(css, false);
    while !processor.end_of_file() {
      processor.next_token(false);
    }
    let end = start.elapsed();
    println!("rust: tokenizer/{}({}): {:?}", file, size, end);
  }
}

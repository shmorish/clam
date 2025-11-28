use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Editor, Result};

fn main() -> Result<()> {
    let mut rl: Editor<(), FileHistory> = Editor::new()?;
    let history_file = ".clam_history";

    // 履歴ファイルから読み込み
    let _ = rl.load_history(history_file);

    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // 履歴に追加
                let _ = rl.add_history_entry(&line);
                println!("Input: {}", trimmed);
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D (EOF) でループを抜ける
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // 履歴を保存
    rl.save_history(history_file)?;
    Ok(())
}

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use tokio::sync::mpsc;

// --- データの構造体 ---

#[derive(Debug, Deserialize)]
struct InputRecord {
    timestamp: String,
    #[serde(rename = "type")]
    record_type: String, // 'type'はRustのキーワードなので'record_type'に変更
    branch: String,
    account: u64,
}

#[derive(Debug, Serialize)]
struct OutputRecord {
    timestamp: String,
    branch_account: String,
}

// --- メインロジック ---

/// ファイルAを読み込み、新しく追記された行を処理してファイルBに結果を追記します。
fn process_new_lines(
    input_path: &Path,
    output_path: &Path,
    current_offset: &mut u64,
) -> io::Result<()> {
    // ファイルAを読み込み用に開く
    let mut file_a = File::open(input_path)?;
    // ファイルの現在サイズを取得
    let file_size = file_a.metadata()?.len();

    // ファイルサイズが前回処理したオフセットより小さければ何もしない（ファイルがリセットされた等の場合はここでは考慮しない）
    if file_size <= *current_offset {
        return Ok(());
    }

    // 前回読み込んだ位置にシークする
    file_a.seek(SeekFrom::Start(*current_offset))?;

    // 新しく追加されたバイト列だけを読み込む
    let mut reader = BufReader::new(file_a.take(file_size - *current_offset));
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;

    // ファイルBを出力（追記）用に開く
    let mut file_b = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(output_path)?;

    // 読み込んだ文字列を改行で分割し、1行ずつ処理
    for line in buffer.lines() {
        if line.trim().is_empty() {
            continue;
        }

        match from_str::<InputRecord>(line) {
            Ok(record) => {
                // 処理ロジック: 'type'が"mod"のレコードのみを対象とする
                if record.record_type == "mod" {
                    let branch_account = format!("{}-{}", record.branch, record.account);
                    let output = OutputRecord {
                        timestamp: record.timestamp,
                        branch_account,
                    };

                    // JSON形式(1行)にシリアライズし、改行を追加してファイルBに書き込む
                    let output_json = to_string(&output).unwrap_or_default();
                    writeln!(file_b, "{}", output_json)?;
                }
            }
            Err(e) => {
                eprintln!("Error parsing JSON line: '{}'. Error: {}", line, e);
                // 不正な行でも処理を継続するため、ここではエラーを返さずログ出力のみ
            }
        }
    }

    // 処理が完了したら、オフセットをファイルの新しいサイズに更新する
    *current_offset = file_size;

    Ok(())
}

#[tokio::main]
async fn main() -> notify::Result<()> {
    // --- ファイルパス設定 ---
    let input_file_path_str = "file_a.txt";
    let output_file_path_str = "file_b.txt";

    let output_path = Path::new(output_file_path_str);

    // 監視対象のファイルが存在しない場合、touchコマンドのように空のファイルを作成する
    if !Path::new(input_file_path_str).exists() {
        println!("Input file not found. Creating an empty file: {}", input_file_path_str);
        File::create(input_file_path_str)?;
    }

    // 監視対象ファイルの絶対パスを取得する
    let input_path = std::fs::canonicalize(input_file_path_str)?;

    println!(
        "Watching file: {}",
        input_path.display()
    );
    println!(
        "Output file: {}",
        output_path.display()
    );

    // --- 初期オフセットの設定と初期処理 ---
    // 監視開始前に、ファイルAが存在すれば一度全行を処理し、オフセットを終端に設定する
    let mut current_offset: u64 = 0;
    match process_new_lines(&input_path, output_path, &mut current_offset) {
        Ok(_) => println!("Initial file processing complete. Offset set to {}.", current_offset),
        Err(e) => eprintln!("Initial file processing failed: {}", e),
    }
    // --- ファイル監視のセットアップ ---

    // notifyからのイベントを受け取るためのチャネルを作成
    let (tx, mut rx) = mpsc::channel(100);

    // ウォッチャーを作成 (非同期ランタイムで動作させる)
    let config = Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut watcher = RecommendedWatcher::new(
        move |res: std::result::Result<Event, notify::Error>| {
            // イベントをチャネル経由で送信
            if let Err(e) = tx.blocking_send(res) {
                eprintln!("Error sending event: {}", e);
            }
        },
        config,
    )?;

    // 監視対象のパスを設定
    watcher.watch(&input_path, RecursiveMode::NonRecursive)?;

    // --- イベントループ ---
    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                // ファイルAに対する変更イベントのみを処理する
                // ファイルへの追記は Modify(Data) として通知されることが多い
                use notify::event::{EventKind, ModifyKind};
                match event.kind {
                    EventKind::Modify(ModifyKind::Data(_)) | EventKind::Create(_) => {
                        if event.paths.contains(&input_path) {
                            println!("Detected file modification for {:?}: {:?}", input_path.file_name().unwrap_or_default(), event.kind);
                            match process_new_lines(&input_path, output_path, &mut current_offset) {
                                Ok(_) => {
                                    println!("Processing complete. New offset: {}", current_offset);
                                }
                                Err(e) => {
                                    eprintln!("Error during file processing: {}", e);
                                }
                            }
                        }
                    }
                    _ => { /* その他のイベントは無視 */ }
                }
            }
            Err(e) => {
                eprintln!("Watch error: {:?}", e);
            }
        }
    }

    Ok(())
}
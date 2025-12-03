# 課題

ファイルA（JSON形式）をRustのnotifyクレートを利用して読み込み続け、ファイルB（JSON形式）を出力し続けるプログラムをRustで実装する。

## 仕様

### 入力 (ファイルA)
- 形式: JSON Lines (`.jsonl` または `.log` などを想定)
- 各行のJSONオブジェクトは以下のフィールドを持つ。
  - `id`: `string`
  - `timestamp`: `string`
  - `type`: `string` ("new", "mod" など)
  - `branch`: `string`
  - `account`: `number`

### 出力 (ファイルB)
- 形式: JSON Lines
- 入力データのうち、`"type": "mod"` の行のみを処理対象とする。
- 処理対象の行から、以下の形式で新しいJSONオブジェクトを生成し、ファイルBに追記する。
  - `timestamp`: 入力の `timestamp` をそのまま使用。
  - `branch_account`: 入力の `branch` と `account` をハイフン `-` で連結した文字列。

## 動作例

### 1. 初期状態

**ファイルAの内容:**
```json
{"id": "userA", "timestamp": "hhmms1", "type": "new", "branch": "ZZA", "account": 123456}
{"id": "userA", "timestamp": "hhmms2", "type": "mod", "branch": "ZZA", "account": 123456}
{"id": "userB", "timestamp": "hhmms3", "type": "new", "branch": "ZZB", "account": 123456}
```

プログラムを起動すると、`"type": "mod"` の行が処理され、**ファイルBに以下の内容が出力される。**
```json
{"timestamp": "hhmms2", "branch_account": "ZZA-123456"}
{"timestamp": "hhmms3", "branch_account": "ZZB-123456"}
```

### 2. ファイルAへの追記

プログラムを稼働させたまま、**ファイルAに以下の3行を追記する。**
```json
{"id": "userC", "timestamp": "hhmms4", "type": "new", "branch": "ZZC", "account": 123456}
{"id": "userB", "timestamp": "hhmms5", "type": "mod", "branch": "ZZB", "account": 123456}
{"id": "userC", "timestamp": "hhmms6", "type": "mod", "branch": "ZZC", "account": 123456}
```

追記された `"type": "mod"` の行が処理され、**ファイルBに以下の内容が追記される。**
```json
{"timestamp": "hhmms5", "branch_account": "ZZB-123456"}
{"timestamp": "hhmms6", "branch_account": "ZZC-123456"}
```

https://github.com/user-attachments/assets/cb91556a-ae9f-4654-ba43-33a94c38116d

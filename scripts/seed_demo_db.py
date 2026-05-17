#!/usr/bin/env python3
from __future__ import annotations

import random
import sqlite3
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DB_PATH = ROOT / 'deeplx-monitor.db'

LANG_PAIRS = [
    ('EN', 'ZH'), ('JA', 'ZH'), ('ZH', 'EN'), ('KO', 'ZH'),
    ('FR', 'EN'), ('DE', 'ZH'), ('ES', 'EN'), ('RU', 'ZH'),
    ('PT', 'EN'), ('IT', 'ZH'), ('TR', 'EN'), ('AR', 'ZH'),
]
ERRORS = [
    'Upstream timeout after 10s',
    'HTTP 429 Too Many Requests',
    'TLS handshake failed',
    'Invalid JSON response from upstream',
]


def ensure_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(
        '''
        CREATE TABLE IF NOT EXISTS stats_anchor (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            total_requests INTEGER NOT NULL DEFAULT 0,
            total_chars INTEGER NOT NULL DEFAULT 0,
            anchor_date TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS translation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chars INTEGER NOT NULL,
            source_lang TEXT NOT NULL,
            target_lang TEXT NOT NULL,
            status TEXT NOT NULL,
            error_msg TEXT,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            source_chars INTEGER NOT NULL DEFAULT 0,
            target_chars INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_created_at ON translation_logs(created_at);
        INSERT OR IGNORE INTO stats_anchor (id, total_requests, total_chars, anchor_date)
        VALUES (1, 0, 0, date('now', 'localtime'));
        '''
    )


def main() -> None:
    random.seed(20260511)
    conn = sqlite3.connect(DB_PATH)
    ensure_schema(conn)

    conn.execute('DELETE FROM translation_logs')
    conn.execute('DELETE FROM sqlite_sequence WHERE name = ?', ('translation_logs',))
    conn.execute(
        'UPDATE stats_anchor SET total_requests = 0, total_chars = 0, anchor_date = date(\'now\', \'localtime\') WHERE id = 1'
    )

    now = datetime.now().replace(minute=0, second=0, microsecond=0)
    rows: list[tuple[int, str, str, int, int, str, str | None, str]] = []

    for day_offset in range(30):
        base_day = now - timedelta(days=29 - day_offset)
        samples_per_day = 10 if day_offset >= 24 else 6
        for sample_idx in range(samples_per_day):
            source_lang, target_lang = LANG_PAIRS[(day_offset + sample_idx) % len(LANG_PAIRS)]
            hour = (sample_idx * 3 + day_offset) % 24
            minute = (sample_idx * 11) % 60
            created_at = base_day.replace(hour=hour, minute=minute)

            source_chars = 80 + ((day_offset * 37 + sample_idx * 19) % 620)
            target_chars = max(40, int(source_chars * (0.85 + ((sample_idx % 5) * 0.06))))
            is_error = sample_idx == 0 and day_offset % 5 == 0
            status = 'error' if is_error else 'success'
            error_msg = random.choice(ERRORS) if is_error else None

            rows.append(
                (
                    source_chars,
                    source_lang,
                    target_lang,
                    source_chars,
                    target_chars,
                    status,
                    error_msg,
                    created_at.strftime('%Y-%m-%d %H:%M:%S'),
                )
            )

    # 补足最近 24 小时的小时级趋势，方便把小时图表撑满
    recent_langs = [('EN', 'ZH'), ('JA', 'ZH'), ('ZH', 'EN'), ('DE', 'ZH')]
    for hour_offset in range(24):
        created_at = now - timedelta(hours=23 - hour_offset)
        source_lang, target_lang = recent_langs[hour_offset % len(recent_langs)]
        source_chars = 120 + (hour_offset * 23) % 520
        target_chars = source_chars + (hour_offset % 7) * 9
        status = 'success' if hour_offset % 6 else 'error'
        error_msg = None if status == 'success' else 'Synthetic upstream error for UI QA'
        rows.append(
            (
                source_chars,
                source_lang,
                target_lang,
                source_chars,
                target_chars,
                status,
                error_msg,
                created_at.strftime('%Y-%m-%d %H:%M:%S'),
            )
        )

    conn.executemany(
        '''
        INSERT INTO translation_logs (
            chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ''',
        rows,
    )
    conn.commit()

    total_requests = conn.execute('SELECT COUNT(*) FROM translation_logs').fetchone()[0]
    total_chars = conn.execute('SELECT COALESCE(SUM(chars), 0) FROM translation_logs').fetchone()[0]
    print(f'Seeded demo DB: {DB_PATH}')
    print(f'  rows={total_requests}')
    print(f'  total_chars={total_chars}')


if __name__ == '__main__':
    main()

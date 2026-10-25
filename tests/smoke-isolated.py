#!/usr/bin/env python3
"""Exercise the unchanged upstream Rust file-copy code on synthetic data, never game files.
Linux host only: build --locked --release --bin replacer --features ae --target x86_64-unknown-linux-gnu.
Windows DLL/loader behaviour is not exercised by this script.
"""
from pathlib import Path
import json
import subprocess
import sys
import tempfile

exe = Path(sys.argv[1]).resolve()
results = []
with tempfile.TemporaryDirectory(prefix="mmr-smoke-") as td:
    root = Path(td)
    def case(name, backup=False, source_state="normal"):
        base = root/name
        base.mkdir()
        source, dest, copies = base/'source', base/'destination', base/'backups'
        dest.mkdir()
        if source_state != "missing":
            source.mkdir()
        if source_state == "normal":
            replacement = source/'candidate'/'data'
            (replacement/'meshes').mkdir(parents=True)
            (replacement/'textures').mkdir()
            (replacement/'meshes'/'marker.txt').write_text('replacement')
            (replacement/'textures'/'ignored.dds').write_text('ignore-me')
        (dest/'meshes').mkdir()
        (dest/'meshes'/'marker.txt').write_text('original')
        config = f"Filter = '[.]txt$'\nLogLevel = 'Trace'\nDestination = '{dest}'\nAnniversaryWorkaroud = false\n"
        if backup:
            config += f"[Backup]\nDirectory = '{copies}'\nFilter = 'Hash'\n"
        config += f"[[Source]]\nDirectory = '{source}'\n"
        # Upstream uses Windows-style relative strings; Linux treats each as a literal filename.
        (base/r'.\data\skse\plugins\MainMenuRandomizer.config.toml').write_text(config)
        run = subprocess.run([str(exe)], cwd=base, text=True, capture_output=True)
        assert run.returncode == 0, (name, run.stdout, run.stderr)
        log = (base/r'.\data\skse\plugins\MainMenuRandomizer.log').read_text()
        if source_state == 'normal':
            assert (dest/'meshes'/'marker.txt').read_text() == 'replacement'
            assert not (dest/'textures'/'ignored.dds').exists()
            if backup:
                stored = list(copies.glob('*/meshes/marker.txt'))
                assert len(stored) == 1 and stored[0].read_text() == 'original'
                assert (copies/'hashes').exists()
                assert subprocess.run([str(exe)],cwd=base).returncode == 0
                assert len(list(copies.glob('*/meshes/marker.txt'))) == 1
                assert stored[0].read_text() == 'original'
        else:
            assert (dest/'meshes'/'marker.txt').read_text() == 'original'
            expected = 'does not exist' if source_state == 'missing' else 'is empty'
            assert expected in log
        results.append({'case':name,'passed':True})
    case('copy-and-filter')
    case('backup-hash-preserved', backup=True)
    case('missing-source', source_state='missing')
    case('empty-source', source_state='empty')
print(json.dumps({'host':'Linux synthetic filesystem only','runtime_tested':False,'results':results},indent=2))

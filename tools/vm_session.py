"""Known VM session locations; never discovers or attaches physical drives."""
from pathlib import Path
import qemu
ROOT=Path(__file__).resolve().parents[1]
VERSION='0.4.0'
LEGACY_VERSION='0.3.0'

def interactive(version=VERSION):return ROOT/'test-output'/('interactive-vm-'+version)

def running_session(require_current=False):
    original=(qemu.OUT,qemu.STATE)
    try:
        for version in [VERSION,LEGACY_VERSION]:
            directory=interactive(version);qemu.OUT=directory;qemu.STATE=directory/'qemu-session.json'
            try:qemu.command('query-status')
            except (OSError,ValueError,FileNotFoundError):continue
            if require_current and version!=VERSION:
                raise ValueError('Native export needs the updated VM. Press Q in the old VM, then run UPGRADE NULLGRAV VM.cmd. Its vault is preserved.')
            return directory
    finally:qemu.OUT,qemu.STATE=original
    raise ValueError('Start TEST NULLGRAV VM.cmd first.')

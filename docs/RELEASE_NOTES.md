NULLGRAV 0.3.1 adds a Windows file picker for loading files into a running test VM.

Double-click ADD FILES TO NULLGRAV VM.cmd, select your files, then in NULLGRAV:
create/unlock a vault with N/U; press 3 and R; select the last non-boot FAT volume;
press Enter, select a file, and press Enter to encrypt/import it. Press 2 to view.
No second physical USB stick or VM restart is needed.

The virtual transfer drive is read-only, originals stay unchanged, and the latest
transfer is reattached after restart. Transfer images contain plaintext copies on
the Windows computer; only the imported vault copy is encrypted. Selected files
are never included in this public release. HOW TO ADD FILES.txt has the steps.

The native OS image is unchanged from 0.3.0. This patch retains the existing VM
vault and its versioned directory. The image remains unsigned and unaudited.
Three failed native unlocks still erase the entire boot medium. Do not guess a
forgotten passphrase. Physical J3455/SD hardware remains untested.

Validation: focused Python transfer-helper tests, native live-USB hotplug,
file import and viewing, unchanged originals/transfer images/other VM disks,
vault preservation and read-only transfer restoration after restart, and the
actual Windows PowerShell-to-WSL import path. No physical disk was passed to QEMU.

Keep files below 128 MiB each. A batch can contain up to 200 files / 200 MiB;
the current preview vault holds about 250 MiB total, including earlier imports.
Images/text open directly; PDF and ordinary video need preparation for native
viewing. Python 3 on Windows and the existing Ubuntu WSL VM setup are required.

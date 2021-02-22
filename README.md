# Introduction

Rdump is a system backup manager.  It isn't backup software itself,
but is able to use strategies from the underlying system to create and
maintain reliable backups.  It is able to make use of LVM2
(snapshots), features of ZFS (snapshots, send/recv), and actual backup
software, such as borgbackup to make these backups.

Rdump incorporates the file integrity tools from rsure so that it is
possible to perform test restores of backups and ensure that the data
will be properly restored.

# Strategies

For most systems, several strategies are needed to be able to perform
reliable backups.  Some of these strategies:

- Regular Linux filesystems.  These are backed up as well as possible,
  given that there are no snapshots.  Because of the possibility that
  filesystem changes will produce an inconsistent backup, it is
  recommended to only use this strategy when there is no choice, such
  as for boot and EFI partitions.  Fortunately, for these partitions,
  there won't be many things modifying these filesystems during the
  backup.

- LVM2.  We are able to use LVM2 snapshots to perform backups of
  regular linux filesystems that are on LVM2 volumes.  The approach is
  generally along the lines of:

  - Perform an lvm2 snapshot of the volume.
  - Run an rsure update to update the `2sure.dat.gz` file within the
    snapshot.
  - Copy the `2sure.dat.gz` file back to the original volume, so that
    copy is the most up to date.
  - Use borgbackup to back up the snapshot and/or use rsync to
    synchronize the snapshot with a ZFS volume
  - Destroy the snapshot

- ZFS.  ZFS has a lot of tools for making snapshots/clones.  Please
  see the section below on some interactions between ZFS snapshots and
  rsure that we have to be careful with.  ZFS volumes may be primary,
  meaning they are working directories containing live data, or they
  may be clones of a filesystem not originating from ZFS (using rsync,
  see above).  In both cases, backing these up is similar:

  - Use `zfs snapshot` to make a snapshot.
  - Use `zfs clone` to make a writable clone of that snapshot.
  - Update the integrity data within the clone.
  - Copy the `2sure.dat.gz` file back to the original volume.
  - Optinally produce an `-rsure` snapshot containing this updated
    integrity data
  - Optionally use borgbackup to back up this snapshot
  - Optionally clone the ZFS snapshots to another location

# Backup tools

Rdump does not try to perform backups itself.  It makes use of
existing backup software.  The following are some options and why they
are or are not used.

- GNU Tar.  GNU tar is a ubuiquitous tool for archiving files.
  However, it is less than useful for backup, mostly because it
  incremental implementation is poor (as in, restored backups are
  unlikely to have the same files as the filesystem when the backup
  was made).

- STAR.  Star seems to do incremental (level) backups and restore
  correctly.  However, initial testing found the restore to be quite
  slow.  This may still be a useful backup strategy, as the backups
  are reasonably fast.

- dumpe2fs.  Dumpe2fs requires that the filesystem being backed up be
  idle.  This is reasonable when lvm2 snapshots.  Its management of
  backup levels and destinations seems to assume that there will only
  be one backup destination, and managing this will require manual
  manipulation of the /etc/dumpdates file.  It only works with the
  ext[2-4] filesystems.  I have not tested restores.

- xfsdump.  xfsdump dumps live filesystems and is supposed to be
  robust.  My experiences testing with it have shown occasional
  backups that cannot be restored, or files missing from the restore.

- borgbackup.  Borg (previously called Attic) is a content-addressible
  backup tool.  It is fairly mature, efficient, and in my testing
  produces robust and reliable backups.  The destination is written to
  a series of files that are only ever newly created.  It supports
  encryption (and encourages it, making unencrypted backups
  difficult).

- restic.  Restic is another content-addressible backup tool, written
  in Go.  It is less mature than borg backup.

- rsync.  Rsync is a tool to synchronize a tree between two locaitons.
  It supports delta updates.  In general, it is fairly robust, as long
  as the proper arguments are given.  However, it only uses the
  'mtime' field to compare trees, and if tools change the mtime of a
  file (and also don't change the size), rsync will miss updates to
  files.  In my experience, the only time I've ever had that matter
  was with system updates where the packaging had recompressed the
  same file.  Although the uncompressed was identical, there was a
  timestamp that differed (of when the compression was run).  In other
  words, the backup was technically incorrect, although the end result
  was unlikely to matter.

There are other tools, such as rdiff-backup that write to a
filesystem, which can be useful for instances.  Although, with ZFS as
a destination, rsync itself is probably adequate, along with
snapshots.

# ZFS snapshots and rsure

LVM2 snapshots are generally COW in both directions (the original can
still be modified, but the snapshot itself can also be modified.  We
take advantage of this when performing an integrity scan to have the
updated surefile as part of the backup.

With ZFS snapshots, this isn't really possible, since snapshots in ZFS
are read-only.

ZFS does support the concept of a clone, which is a writable version
of the snapshot.  We can use this to both provide a way to write the
sure data within the clone, and to back up the data as well.

To do this, we'll do something like:

```
zfs snapshot pool/volume@snapname
```

to create the initial snapshot, followed by

```
zfs clone pool/volume@snapname pool/snaps/volume
```

We can then perform an integrity update of /pool/snaps/volume,
followed, and then copy that integrity data back to the original.
Then, we can back up this snapshot (which, conveniently will now also
have a consistent path, which borg needs).  Finally, the clone can be
destroyed, freeing up any space.

The clones themselves won't have up-dated snapshot information, and
there are a few things we could do.  One possibility would be, after
copying the rsure data back into the original volume, make another
snapshot there, say pool/volume@snapname-rsure, perhaps even write a
readme file to explain that the snapshot is just to update the rsure
file, and that the filesystem itself may not be consistent with that
data.

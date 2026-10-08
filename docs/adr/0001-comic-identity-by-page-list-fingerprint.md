# Identify a comic by a fingerprint of its page list

Reading progress is keyed by a fingerprint built from the comic's sorted page names and byte sizes, with any common leading directory removed. We rejected an MD5 of the archive file plus the folder path for folders: hashing a whole archive is slow on large files, any repack changes the hash, and a folder path breaks when the folder is moved or renamed. The fingerprint is cheap (names and sizes are already listed at open), works the same for archives and folders, and survives move, rename and repack. Editing, adding or removing a page changes the fingerprint; a fallback matches the last known source path when the saved page name still exists.

Changing this scheme later orphans all stored progress, so treat it as hard to reverse.

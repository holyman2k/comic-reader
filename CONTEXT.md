# Comic Reader

A desktop viewer that shows a comic as one continuous vertical scroll of pages, and remembers where the reader stopped.

## Language

**Comic**:
A sequence of page images that a reader treats as one work, stored as an archive file or a folder.
_Avoid_: Book (code term for an opened comic), title, issue

**Source**:
The archive file (zip, cbz, rar, cbr, tar.gz) or folder that holds a comic's pages.
_Avoid_: Path, container

**Page**:
One image of a comic. Pages are ordered by natural sort of their names.
_Avoid_: Image, slide

**Progress**:
The last page a reader reached in a comic, plus whether the reader finished it.
_Avoid_: Bookmark, position, history

**Reached page**:
The page at the top edge of the viewport. This is the page saved as progress.
_Avoid_: Current page (the toolbar counter uses the viewport center, a different measure)

**Finished**:
A comic is finished when the reader has scrolled to the bottom of its last page. Reaching a later page of a finished comic after reopening clears this state.
_Avoid_: Completed, read

**Fingerprint**:
The identity of a comic, derived from its list of pages (names and sizes) and not from where its source lives. A moved, renamed or repacked source keeps the same fingerprint as long as its pages are unchanged.
_Avoid_: Hash, ID, checksum

**Resume**:
Opening a comic whose fingerprint has stored progress and jumping to its reached page. A finished comic opens at its first page instead.
_Avoid_: Restore, continue

export interface PageInfo {
  name: string;
  width: number | null;
  height: number | null;
}

export interface Resume {
  pageIndex: number;
  finished: boolean;
}

export interface BookInfo {
  bookId: number;
  title: string;
  pages: PageInfo[];
  fingerprint: string;
  resume: Resume | null;
}

/** Error string from open_book when a newer open replaced the request. */
export const SUPERSEDED = "superseded";

export interface PageInfo {
  name: string;
  width: number | null;
  height: number | null;
}

export interface BookInfo {
  bookId: number;
  title: string;
  pages: PageInfo[];
}

/** Error string from open_book when a newer open replaced the request. */
export const SUPERSEDED = "superseded";

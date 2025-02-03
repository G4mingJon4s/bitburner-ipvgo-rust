import type { Go } from "./NetscriptDefinitions";

type ChainData = {
  tiles: number[];
  /** Includes liberties */
  adjacent: number[];
} & ({
  type: "White" | "Black";
  liberties: number[];
} | {
  type: "Dead" | "Free";
});

type WhiteChainData = ChainData & { type: "White" };
type BlackChainData = ChainData & { type: "Black" };

type PreviousData = {
  move: number | "pass";
  board: bigint;
  prev: PreviousData | null;
};

export class Board {
  // Constants representing tile types.
  static readonly DEAD = "#";
  static readonly WHITE = "O";
  static readonly BLACK = "X";
  static readonly FREE = ".";

  static toCharacter(rep: "White" | "Black" | "Dead" | "Free") {
    if (rep === "White") return Board.WHITE;
    if (rep === "Black") return Board.BLACK;
    if (rep === "Dead") return Board.DEAD;
    return Board.FREE;
  }

  // Bitmaps for white routers, black routers, and dead nodes.
  white: Uint32Array;
  black: Uint32Array;
  dead: Uint32Array;

  // Board size (assuming a square board with size x size cells).
  size: number;

  // The current turn ("White" or "Black").
  turn: "White" | "Black" | "None";

  // Reference to the previous board state and the move that lead to this board
  previous: PreviousData | null = null;;

  // game-specific metadata, e.g., komi.
  komi: number = 0;

  // Cached data
  private _data: {
    chains: {
      /** Unordered */
      all: ChainData[];
      /** Ordered, pos[i] is the chain at pos i */
      pos: ChainData[];
      /** Unordered */
      white: WhiteChainData[];
      /** Unordered */
      black: BlackChainData[];
    };
  } | null = null;

  get data(): NonNullable<Board["_data"]> {
    if (this._data !== null) return this._data;

    const all: ChainData[] = [];
    const pos = Array.from<ChainData>({ length: this.size * this.size });
    const white: WhiteChainData[] = [];
    const black: BlackChainData[] = [];

    const seen = new Set<number>();
    for (let i = 0; i < this.size * this.size; i++) {
      if (seen.has(i)) continue;
      const chain = this.getChain(i);
      chain.tiles.forEach(t => {
        seen.add(t);
        pos[t] = chain;
      });
      all.push(chain);
      // The type assertion is needed, sorry
      if (chain.type === "White") white.push(chain as any);
      if (chain.type === "Black") black.push(chain as any);
    }

    if (pos.some(a => a === undefined)) throw new Error("Missed tiles");

    this._data = {
      chains: {
        all,
        pos,
        white,
        black,
      },
    };
    return this._data;
  }

  constructor(size: number, turn: "White" | "Black" | "None") {
    this.size = size;
    this.turn = turn;
    const totalCells = size * size;
    // Calculate number of 32-bit integers needed.
    const idx = Math.ceil(totalCells / 32);
    this.white = new Uint32Array(idx);
    this.black = new Uint32Array(idx);
    this.dead = new Uint32Array(idx);
  }

  private copy(): Board {
    const board = new Board(this.size, this.turn);
    board.white.set(this.white);
    board.black.set(this.black);
    board.dead.set(this.dead);
    board.komi = this.komi;
    board.previous = this.previous;
    return board;
  }

  /**
   * Converts a board cell position (0-indexed linear index) to its array index and bit offset.
   *
   * @param pos - The linear position (0 to size*size - 1).
   * @returns A tuple [array index, bit offset].
   */
  private index(pos: number): [number, number] {
    return [Math.floor(pos / 32), pos % 32];
  }

  private get(arr: Uint32Array, pos: number): boolean {
    const [idx, ofs] = this.index(pos);
    const value = arr[idx] & (1 << ofs);
    return value !== 0;
  }

  private set(arr: Uint32Array, pos: number): void {
    const [idx, ofs] = this.index(pos);
    arr[idx] |= (1 << ofs);
  }

  private clear(arr: Uint32Array, pos: number): void {
    const [idx, ofs] = this.index(pos);
    arr[idx] &= ~(1 << ofs);
  }

  private place(pos: number, stone = this.turn) {
    if (stone === "None") throw new Error("Trying to place when game is over");
    this.set(stone === "White" ? this.white : this.black, pos);
    this._data = null;
  }

  private remove(pos: number, stone = this.turn) {
    if (stone === "None") throw new Error("Trying to clear when game is over");
    this.clear(stone === "White" ? this.white : this.black, pos);
    this._data = null;
  }

  private tile(pos: number): "White" | "Black" | "Dead" | "Free" {
    if (this.get(this.white, pos)) return "White";
    if (this.get(this.black, pos)) return "Black";
    if (this.get(this.dead, pos)) return "Dead";
    return "Free";
  }

  static from(
    boardState: string[],
    gameState: ReturnType<Go["getGameState"]>,
    previous: PreviousData | null = null
  ): Board {
    const size = boardState.length;
    if (size === 0 || boardState.some(row => row.length !== size)) {
      throw new Error("Board state must be a non-empty square array of strings.");
    }

    const board = new Board(size, gameState.currentPlayer);
    board.komi = gameState.komi;
    board.previous = previous;

    for (let x = 0; x < size; x++) {
      for (let y = 0; y < size; y++) {
        const pos = x * size + y;
        const char = boardState[x][y];
        if (char === Board.WHITE) {
          board.set(board.white, pos);
        } else if (char === Board.BLACK) {
          board.set(board.black, pos);
        } else if (char === Board.DEAD) {
          board.set(board.dead, pos);
        }
      }
    }
    return board;
  }

  /**
   * Converts the Board back to the in-game representation
   */
  convert(): string[] {
    const rep: string[] = [];

    for (let x = 0; x < this.size; x++) {
      let s = "";
      for (let y = 0; y < this.size; y++) {
        s += Board.toCharacter(this.tile(x * this.size + y));
      }
      rep.push(s);
    }

    return rep;
  }

  hash(): bigint {
    const rep = this.convert().join("\n");
    let hash = BigInt(0);
    for (let x = 0; x < rep.length; x++) {
      for (let y = 0; y < rep.length; y++) {
        const char = rep[x][y];
        hash = (hash << BigInt(2)) | BigInt(char === Board.WHITE
          ? 0b00
          : char === Board.BLACK
          ? 0b01
          : char === Board.DEAD
          ? 0b10
          : 0b11
        );
      }
    }
    return hash;
  }

  /**
   * Returns the board representation in the correct format
   */
  static display(rep: Board | string[]): string[] {
    if (rep instanceof Board) rep = rep.convert();

    const out: string[] = [];
    rep = rep.map(r => r.split("").toReversed().join(""));
    for (let y = 0; y < rep.length; y++) {
      let row = "";
      for (let x = 0; x < rep.length; x++) {
        row += rep[x][y];
      }
      out.push(row);
    }
    return out;
  }

  matches(other: Board): boolean {
    const thisRep = this.convert();
    const otherRep = other.convert();

    if (thisRep.length !== otherRep.length) return false;
    return thisRep.every((l, x) => otherRep[x] === l);
  }

  /**
   * Simulates making a move, returns a new Board
   */
  makeMove(pos: number): Board | null {
    if (this.turn === "None") return null;
    if (this.tile(pos) !== "Free") return null;

    const copy = this.copy();
    copy.place(pos);

    const nextPlayer = this.turn === "White" ? "Black" : "White";
    copy.turn = nextPlayer;
    copy.previous = {
      board: this.hash(),
      move: pos,
      prev: this.previous
    };

    const affected = this.neighbors(pos);
    for (const tile of affected) {
      const chain = copy.getChain(tile);
      if (chain.type !== nextPlayer) continue;
      if (chain.liberties.length === 0) for (const t of chain.tiles) copy.remove(t, nextPlayer);
    }

    const chain = copy.getChain(pos);
    if (chain.type !== this.turn) throw new Error("Placement error. This is a bug.");
    if (chain.liberties.length === 0) return null;

    const hash = copy.hash();
    let cur: PreviousData | null = this.previous;
    while (cur !== null) {
      if (hash === cur.board) return null;
      else cur = cur.prev;
    }

    return copy;
  }

  *validMoves(): Generator<[number | "pass", Board], void> {
    yield ["pass", this.makePass()];
    for (let i = 0; i < this.size * this.size; i++) {
      const board = this.makeMove(i);
      if (board === null) continue;
      yield [i, board];
    }
  }

  /**
   * Simulates passing, returns a new Board
   */
  makePass(): Board {
    const newBoard = this.copy();

    newBoard.turn = this.previous?.move === "pass"
      ? "None"
      : this.turn === "White"
        ? "Black"
        : "White";
    newBoard.previous = {
      board: this.hash(),
      move: "pass",
      prev: this.previous
    };

    return newBoard;
  }

  /**
   * Gets information about a chain
   */
  getChain(pos: number): ChainData {
    const type = this.tile(pos);

    const tiles = new Set<number>();
    const adjacent = new Set<number>();
    const queue: number[] = [pos];

    while (queue.length !== 0) {
      const cur = queue.shift()!;
      if (tiles.has(cur)) continue;
      tiles.add(cur);

      const possible = this.neighbors(cur).filter(p => {
        if (this.tile(p) === type) return true;

        adjacent.add(p);
        return false;
      });
      queue.push(...possible);
    }

    if (type !== "White" && type !== "Black") return {
      type,
      tiles: Array.from(tiles),
      adjacent: Array.from(adjacent)
    };

    const liberties = Array.from(adjacent).filter(t => this.tile(t) === "Free");
    return {
      type,
      tiles: Array.from(tiles),
      adjacent: Array.from(adjacent),
      liberties
    };
  }

  private neighbors(pos: number): number[] {
    const [x, y] = [Math.floor(pos / this.size), pos % this.size];
    return [
      [x + 1, y],
      [x - 1, y],
      [x, y + 1],
      [x, y - 1]
    ].filter(([x, y]) => x >= 0 && x < this.size && y >= 0 && y < this.size).map(([x, y]) => x * this.size + y);
  }

  // Converts the Board to a plain object that can be transferred.
  toJSON(): Record<string, any> {
    return {
      size: this.size,
      turn: this.turn,
      komi: this.komi,
      // Convert typed arrays to regular arrays for JSON transfer.
      white: Array.from(this.white),
      black: Array.from(this.black),
      dead: Array.from(this.dead),
      // Assuming previous is serializable or can be set to null.
      previous: this.previous,
    };
  }

  // Recreates a Board instance from the serialized data.
  static fromJSON(data: any): Board {
    const board = new Board(data.size, data.turn);
    board.komi = data.komi;
    // Recreate the typed arrays from the regular arrays.
    board.white = new Uint32Array(data.white);
    board.black = new Uint32Array(data.black);
    board.dead = new Uint32Array(data.dead);
    board.previous = data.previous;
    return board;
  }
}
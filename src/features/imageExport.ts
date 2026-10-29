import { saveUserFile } from "./files";
export const cardColors = { mint: "#40d6b6", blue: "#69adff", violet: "#b497ff", rose: "#ff8fb5", amber: "#f2be63" };
export function canvasPng(canvas: HTMLCanvasElement): Promise<Blob> { return new Promise((resolve,reject)=>canvas.toBlob(blob=>blob?resolve(blob):reject(Error("png")),"image/png")); }
export async function outputCard(canvas: HTMLCanvasElement, name: string, copy: boolean): Promise<boolean> {
  const blob=await canvasPng(canvas);
  if(copy){if(!navigator.clipboard?.write||typeof ClipboardItem==="undefined")throw Error("clipboard");await navigator.clipboard.write([new ClipboardItem({"image/png":blob})]);return true;}
  return saveUserFile(name,new Uint8Array(await blob.arrayBuffer()),"png");
}

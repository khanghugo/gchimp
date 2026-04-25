import { ChangeEvent, createRef, FormEvent, useEffect, useState } from "react";
import { GchimpProgram } from "..";

import "./styles.css";
import { UploadButton } from "@/components/upload-button";
import { useGChimp } from "@/hooks/useGChimp";
import { bsp2wad } from "gchimp-web";

export const Bsp2Wad = () => {
    const { isLoading, fileToBytes } = useGChimp();

    const [name, setName] = useState<string | undefined>(undefined);
    const [file, setFile] = useState<File | null>(null);
    const [output, setOutput] = useState<Uint8Array | null>(null);

    const submitButton = createRef<HTMLInputElement>();

    const runProgram = async (e: FormEvent<HTMLFormElement>) => {
        e.preventDefault();

        if (!file || !name) return;

        try {
            const bytes = await fileToBytes(file);

            const res = bsp2wad(bytes);
            setOutput(res);
        } catch (err) {
            console.error("WASM execution failed:", err);
        }
    };

    const changeFile = (e: ChangeEvent<HTMLInputElement>) => {
        const file = (e.target as HTMLInputElement).files?.item(0);
        // the path will be sandboxed so we only care about the file stem
        setName(file?.name);
        setFile(file ? file : null);
    }

    const onDrop = (e: React.DragEvent<HTMLElement>) => {
        e.preventDefault();

        const file = e.dataTransfer.files.item(0);

        setName(file?.name);

        setFile(file ? file : null);
    }

    const downloadOutputFile = () => {
        if (!output)
            return;

        // tried and true method
        const blob = new Blob([output as BlobPart], { type: 'application/octet-stream' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');

        link.href = url;

        console.assert(name, "no file name");
        if (name)
            link.download = `${extract_file_name(name)}.wad`;

        link.click();

        link.remove();
    }

    // when new file is selected, run the program right away
    useEffect(() => {
        // check the files
        if (!name || (name && !name.endsWith(".bsp")) || !file || !submitButton.current) {
            setName(undefined);
            setFile(null);
            setOutput(null);
            return
        }

        // equivalent to clicking the run button
        submitButton.current?.click();
    }, [
        file, submitButton, name
    ]);

    return <GchimpProgram name="Bsp2Wad" className={`bsp2wad`} onDrop={onDrop} >
        <form onSubmit={async (e) => runProgram(e)}>
            <UploadButton label={"Select or Drop BSP"} id={"bsp2wad-path"} onChange={(e) => changeFile(e)} fileName={name} />
            <div>
                <input type="submit" ref={submitButton} />
                <button type="button" disabled={output === null} onClick={downloadOutputFile}><h2>Get WAD</h2></button>
            </div>
        </form>
    </GchimpProgram>
}

// input is usually `C:\fake_folder\map_name.bsp`
// remember front slash like windows
const extract_file_name = (s: string): string => {
    const splits = s.split("\\");
    const stem = splits[splits.length - 1];
    const file_name = stem.split(".")[0];

    return file_name;
}
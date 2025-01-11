import { ChangeEvent, createRef, FormEvent, useEffect, useState } from "react";
import { GchimpProgram } from "..";

import "./styles.css";
import { split_smd } from "gchimp-web";
import { UploadButton } from "@/components/upload-button";

export const SmdSplit = () => {
    const [name, setName] = useState<string | undefined>(undefined);
    const [file, setFile] = useState<File | null>(null);
    const [output, setOutput] = useState<Uint8Array | null>(null);

    const submitButton = createRef<HTMLInputElement>();

    const runProgram = async (e: FormEvent<HTMLFormElement>) => {
        // dont refresh
        e.preventDefault();

        // reading the file to byte
        const reader = new FileReader();

        reader.onload = (e) => {
            if (name) {
                const file_stem = extract_file_name(name);
                const res = split_smd(new Uint8Array(e.target?.result as ArrayBuffer), file_stem);
                setOutput(res);
            } else {
                console.error("no file name set for input smd file");
            }
        };

        if (!file) {
            // setStatus("No file selected")
            return;
        }

        reader.readAsArrayBuffer(file as Blob);
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
        const blob = new Blob([output], { type: 'application/zip' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');

        link.href = url;

        console.assert(name, "no file name");
        if (name)
            link.download = `${extract_file_name(name)}.zip`;

        link.click();

        link.remove();
    }

    // when new file is selected, run the program right away
    useEffect(() => {
        // check the files
        if (!name || (name && !name.endsWith(".smd")) || !file || !submitButton.current) {
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

    return <GchimpProgram name="SmdSplit" className={`smdsplit`} onDrop={onDrop} >
        <form onSubmit={async (e) => runProgram(e)}>
            <UploadButton label={"Select or Drop SMD"} id={"smdsplit-path"} onChange={(e) => changeFile(e)} fileName={name} />
            <div>
                <input type="submit" ref={submitButton} />
                <button type="button" disabled={output === null} onClick={downloadOutputFile}><h2>Get SMD(s)</h2></button>
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
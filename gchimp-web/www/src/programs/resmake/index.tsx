import { ChangeEvent, createRef, FormEvent, useEffect, useState } from "react";
import { GchimpProgram } from "..";

import "./styles.css";
import { resmake } from "gchimp-web";
import { UploadButton } from "@/components/upload-button";

export const ResMake = () => {
    const [name, setName] = useState<string | undefined>(undefined);
    const [file, setFile] = useState<File | null>(null);
    const [output, setOutput] = useState<string | null>(null);

    const submitButton = createRef<HTMLInputElement>();

    const runResMake = async (e: FormEvent<HTMLFormElement>) => {
        // dont refresh
        e.preventDefault();

        // reading the file to byte then pass it to wave_loop
        const reader = new FileReader();

        reader.onload = (e) => {
            if (name) {
                const res = resmake(new Uint8Array(e.target?.result as ArrayBuffer), name);
                setOutput(res);
            } else {
                console.error("no file name set for input bsp file");
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

    const getResFile = () => {
        if (!output)
            return;

        // tried and true method
        const blob = new Blob([output], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');

        link.href = url;

        console.assert(name, "no file name");
        if (name)
            link.download = `${extract_file_name(name)}.res`;

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

    return <GchimpProgram name="ResMake" className={`resMake`} onDrop={onDrop} >
        <form onSubmit={async (e) => runResMake(e)}>
            <UploadButton label={"Select or Drop BSP"} id={"resmake-path"} onChange={(e) => changeFile(e)} fileName={name} />
            <div>
                <input type="submit" ref={submitButton} />
                <button type="button" disabled={output === null} onClick={getResFile}><h2>Get .RES</h2></button>
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
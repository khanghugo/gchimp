import { ChangeEvent, createRef, FormEvent, useEffect, useState } from "react";
import { GchimpProgram } from "..";

import "./styles.css";
import { dem2cam } from "gchimp-web";
import { UploadButton } from "@/components/upload-button";

export const Dem2Cam = () => {
    const [name, setName] = useState<string | undefined>(undefined);
    const [file, setFile] = useState<File | null>(null);
    const [output, setOutput] = useState<string | null>(null);

    const [overrideFps, setOverrideFps] = useState<string>("0");

    const submitButton = createRef<HTMLInputElement>();

    const runResMake = async (e: FormEvent<HTMLFormElement>) => {
        // dont refresh
        e.preventDefault();

        // reading the file to byte
        const reader = new FileReader();

        reader.onload = (e) => {
            if (name) {
                let fps = Number.parseFloat(overrideFps);

                if (Number.isNaN(fps)) {
                    fps = 0;
                }

                // division by 0 is not an error :DD
                const frametime = 1 / fps;

                const res = dem2cam(new Uint8Array(e.target?.result as ArrayBuffer), name, fps === 0 ? 0 : frametime);

                setOutput(res);
            } else {
                console.error("no file name set for input demo file");
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
            link.download = `${extract_file_name(name)}.cam`;

        link.click();

        link.remove();
    }

    // when new file is selected, run the program right away
    useEffect(() => {
        // check the files
        if (!name || (name && !name.endsWith(".dem")) || !file || !submitButton.current) {
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

    useEffect(() => {
        setName(undefined);
        setFile(null);
        setOutput(null);
    }, [overrideFps])

    return <GchimpProgram name="Dem2Cam" className={`dem2cam`} onDrop={onDrop} >
        <form onSubmit={async (e) => runResMake(e)}>
            <div className="override-fps">
                <label htmlFor="override-fps">Override FPS:</label>
                <input type="text" id="override-fps" value={overrideFps} onChange={(e) => setOverrideFps(e.target.value)} />
            </div>
            <UploadButton label={"Select or Drop Demo"} id={"dem2cam-path"} onChange={(e) => changeFile(e)} fileName={name} />
            <div>
                <input type="submit" ref={submitButton} />
                <button type="button" disabled={output === null} onClick={getResFile}><h2>Get .CAM</h2></button>
            </div>
        </form>
    </GchimpProgram>
}

// input is usually `C:\fake_folder\map_name.dem`
// remember front slash like windows
const extract_file_name = (s: string): string => {
    const splits = s.split("\\");
    const stem = splits[splits.length - 1];
    const file_name = stem.split(".")[0];

    return file_name;
}
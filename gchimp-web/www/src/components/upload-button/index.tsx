import { ChangeEvent, createRef, useState } from "react"

import "./styles.css";
import { Button } from "../button";

interface UploadButtonProps {
    label: string,
    id: string,
    onChange: (e: ChangeEvent<HTMLInputElement>) => void,
    fileName?: string,
}

export const UploadButton = ({ label, id, onChange, fileName }: UploadButtonProps) => {
    // a wrapper button that will trigger input type file
    const hiddenFileInput = createRef<HTMLInputElement>();
    const [file, setFile] = useState<string | null>(null);

    const localOnChange = (e: ChangeEvent<HTMLInputElement>) => {
        setFile(extract_file_name(e.target.value));
        onChange(e);
    };

    return <Button onClick={() => hiddenFileInput.current?.click()} label={label}>
        <input type="file" id={id} ref={hiddenFileInput} onChange={(e) => localOnChange(e)} />
        <p>{fileName ? fileName : file && file}</p>
    </Button>
}

const extract_file_name = (s: string): string => {
    const splits = s.split("\\");
    const stem = splits[splits.length - 1];

    return stem;
}
import "./styles.css";

interface LabelledCheckBoxProps {
    label: string,
    id: string,
    checked: boolean,
    onChange: React.ChangeEventHandler<HTMLInputElement>,
}

export const LabelledCheckBox = ({ label, id, checked, onChange }: LabelledCheckBoxProps) => {
    return <div className="labelled-checkbox">
        <input type="checkbox" id={id} onChange={onChange} checked={checked} />
        <label htmlFor={id}>{label}</label>
    </div>
}
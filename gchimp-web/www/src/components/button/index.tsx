import "./styles.css";

interface ButtonProps {
    label: string,
    onClick: (e: React.MouseEvent<HTMLButtonElement, MouseEvent>) => void,
    children: React.ReactNode,
}

export const Button = ({ label, onClick, children }: ButtonProps) => {
    return <button onClick={(e) => onClick(e)}>
        <h2>{label}</h2>
        {children}
    </button>
}
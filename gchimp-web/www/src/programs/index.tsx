
import "./styles.css";

interface GchimpProgramProps {
    name: string,
    className: string,
    children: React.ReactNode,
    // what a fucking stupid type
    // what kind of idiot came up with typescript?
    onDrop?: (e: React.DragEvent<HTMLElement>) => void,
}

export const GchimpProgram = ({ name, className, children, onDrop }: GchimpProgramProps) => {
    return (
        <section className={className}
            onDrop={onDrop}
            onDragOver={onDrop ? (e) => e.preventDefault() : undefined}
            onDragEnter={onDrop ? (e) => e.preventDefault() : undefined}
        >
            <h1>{name}</h1>
            {children}
        </section>
    );
}
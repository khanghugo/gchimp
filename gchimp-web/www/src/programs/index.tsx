
import "./styles.css";

interface GchimpProgramProps {
    name: string,
    className: string,
    children: React.ReactNode,
    // what a fucking stupid type
    // what kind of idiot came up with typescript?
    onDrop?: (e: React.DragEvent<HTMLElement>) => void,
    dragStartCallback?: () => void,
    dragEndCallback?: () => void,
}

export const GchimpProgram = ({ name, className, children, onDrop, dragStartCallback, dragEndCallback }: GchimpProgramProps) => {
    return (
        <section className={className}
            onDrop={onDrop}
            // need to do this so that the browser won't load the instead
            onDragOver={onDrop ? (e) => e.preventDefault() : undefined}
            onDragEnter={onDrop ? (e) => e.preventDefault() : undefined}
            // nice call back so that css can change
            onDragStart={dragStartCallback ? dragStartCallback : undefined}
            onDragEnd={dragEndCallback ? dragEndCallback : undefined}
        >
            <h1>{name}</h1>
            {children}
        </section>
    );
}
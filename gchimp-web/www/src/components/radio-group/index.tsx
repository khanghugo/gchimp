// vibe coded the whole thing with gemini 2.5 flash, let see

import React from 'react';
import './styles.css'; // Still uses the same CSS file

// Define the structure for each option
interface RadioOption {
    value: string | number;
    label: string;
}

interface RadioGroupProps {
    label?: string; // Make label optional for a group
    name: string; // A name for the radio group (required for grouping)
    options: RadioOption[]; // Array of options to display
    value: string | number | undefined; // The currently selected value
    onChange: (value: string | number) => void; // Handler for when the selection changes
}

export const RadioGroup = ({ label, name, options, value, onChange }: RadioGroupProps) => {
    const handleRadioChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        // Pass the selected value (string or number)
        onChange(event.target.value);
    };

    return (
        <div className="radio-group"> {/* Updated class name */}
            {label && <label className="radio-group-label">{label}</label>} {/* Updated label class name */}
            <div className="radio-options"> {/* Container for the radio buttons */}
                {options.map(option => (
                    <div key={String(option.value)} className="radio-option"> {/* Container for each radio button and label */}
                        <input
                            type="radio"
                            id={`${name}-${String(option.value)}`} // Unique ID based on name and value
                            name={name} // Use the passed name for grouping
                            value={option.value}
                            checked={value === option.value}
                            onChange={handleRadioChange}
                        />
                        <label htmlFor={`${name}-${String(option.value)}`}>{option.label}</label>
                    </div>
                ))}
            </div>
        </div>
    );
};

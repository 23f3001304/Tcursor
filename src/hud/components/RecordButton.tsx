import { motion } from "motion/react";

export function RecordButton({
  disabled,
  onClick,
  label = "Record",
}: {
  disabled: boolean;
  onClick: () => void;
  label?: string;
}) {
  return (
    <motion.button
      className="btn rec"
      onClick={onClick}
      disabled={disabled}
      whileTap={{ scale: 0.97 }}
      transition={{ type: "spring", stiffness: 500, damping: 30 }}
    >
      <span className="dot" />
      {label}
    </motion.button>
  );
}

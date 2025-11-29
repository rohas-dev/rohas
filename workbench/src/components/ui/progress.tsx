import * as React from "react";
import { cn } from "@/lib/utils";

export interface ProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: number;
}

const Progress = React.forwardRef<HTMLDivElement, ProgressProps>(({ className, value = 0, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("relative h-2 w-full overflow-hidden rounded-full bg-muted", className)}
    {...props}
  >
    <div
      className="h-full w-full flex-1 rounded-full bg-primary transition-all"
      style={{ transform: `translateX(-${100 - Math.min(100, Math.max(0, value))}%)` }}
    />
  </div>
));
Progress.displayName = "Progress";

export { Progress };


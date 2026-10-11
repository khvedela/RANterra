import {
  Children,
  isValidElement,
  type ButtonHTMLAttributes,
  type ReactNode,
  type KeyboardEvent,
} from "react";
import {
  Button,
  Select,
  Tooltip,
  Drawer,
  Grid,
  Modal,
  type ModalProps,
} from "antd";

/** Preserve native form submission and existing command handlers while using Ant controls. */
export function ControlButton({
  type = "button",
  className = "",
  title,
  children,
  ...props
}: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "color">) {
  const active = /(?:^|\s)(?:active|chosen)(?:\s|$)/.test(className);
  const button = (
    <Button
      {...props}
      htmlType={type}
      className={className}
      type={
        className.includes("run-button")
          ? "primary"
          : active
            ? "dashed"
            : "default"
      }
      danger={className.includes("delete-entity")}
      aria-label={
        props["aria-label"] ??
        (title && Children.count(children) === 1 ? title : undefined)
      }
    >
      {children}
    </Button>
  );
  return title ? <Tooltip title={title}>{button}</Tooltip> : button;
}

type ValueChange = { target: { value: string } };
type ChoiceProps = {
  children: ReactNode;
  value?: string | number;
  className?: string;
  disabled?: boolean;
  "aria-label"?: string;
  onChange?: (change: ValueChange) => void;
  onFocus?: () => void;
};
/** The existing form handlers consume only a value; no synthetic DOM event is constructed. */
export function SelectField({
  children,
  onChange,
  onFocus,
  value,
  ...props
}: ChoiceProps) {
  const options = Children.toArray(children)
    .filter(isValidElement)
    .map((child) => {
      const option = child.props as {
        value?: string | number;
        children: ReactNode;
      };
      return {
        value: String(option.value ?? option.children),
        label: option.children,
      };
    });
  return (
    <Select
      {...props}
      value={String(value ?? "")}
      options={options}
      onFocus={onFocus}
      onChange={(value) => onChange?.({ target: { value } })}
      popupMatchSelectWidth={false}
      virtual={false}
    />
  );
}

export function WorkspacePanel({
  side,
  open,
  onClose,
  children,
}: {
  side: "left" | "right";
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  const screens = Grid.useBreakpoint();
  if (screens.lg) return <aside className={`${side}-panel`}>{children}</aside>;
  return (
    <Drawer
      title={side === "left" ? "Network setup" : "Entity inspector"}
      placement={side}
      open={open}
      onClose={onClose}
      size={360}
      getContainer={false}
      rootStyle={{ position: "fixed" }}
      className="workspace-drawer"
    >
      <div className={`${side}-panel`}>{children}</div>
    </Drawer>
  );
}

/** Prevent focus from entering browser chrome at the native Tab boundary.
 * Ant 6's focusin guard cannot observe a tab into the browser address bar. */
function containTab(event: KeyboardEvent<HTMLDivElement>) {
  if (event.key !== "Tab") return;
  const elements = Array.from(
    event.currentTarget.querySelectorAll<HTMLElement>(
      "button, input, select, textarea, a[href], [tabindex]",
    ),
  ).filter(
    (el) =>
      el.tabIndex >= 0 &&
      !el.matches(":disabled") &&
      el.getClientRects().length > 0 &&
      getComputedStyle(el).visibility !== "hidden",
  );
  const first = elements[0],
    last = elements.at(-1);
  if (
    (event.shiftKey && document.activeElement === first) ||
    (!event.shiftKey && document.activeElement === last)
  ) {
    event.preventDefault();
    (event.shiftKey ? last : first)?.focus();
  }
}
export function ResearchModal(props: ModalProps) {
  return (
    <Modal
      {...props}
      destroyOnHidden
      // Keep command hit targets stationary from the first visible frame.
      transitionName=""
      maskTransitionName=""
      modalRender={(content) => (
        <div onKeyDownCapture={containTab}>{content}</div>
      )}
    />
  );
}

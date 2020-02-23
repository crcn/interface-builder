Perhaps the best abstraction suited for Paperclip is for the template language to only
be concerned about the view layer. This basically means away any information about the template
that is specific to whatever logic controller is associated with it. For example:

```html
<li id="list-item">
  {#if editing}
  <input id="label-input" />
  {/else}
  <div id="label">{label}</div>
  {/}
</li>
```

☝🏻This is how the previous version of Tandem worked. On the code side, we could do something like this:

```typescript
import ListItem from "./list-item.pc";

export const (props: Props) => {
  const [state, setState] = useState({
    editing: false
  });
  const {editing} = state;

  const onLabelClick = (event: MouseEvent) => {
    setState({ ...state, editing: true });
  };

  const onLabelInputClick = event

  return <ListItem editing={editing} label={label} labelProps={{
    onClick: onLabelClick
  }} labelInputProps={{
    onClick: onLabelInputClick
  }}>;
};
```

☝🏻The API here is slightly clunky, but at least provides a consistent way of adding information to elements. I wonder how this works though with _nested_ components. Take a parent component for example:

```html
<import id="list-item" src="./list-item.pc">
  <ul>
    {#each items as item}
    <list-item item="{item}" />
    {/}
  </ul></import
>
```

More likely we'll want to include a listeners for things like `onRemove`, and `onChange`. So for that we'll need to require nested components to contain IDs _if_ they have logic since there's no way for Paperclip to know what additional properties the logic part of the template needs. This could be accomplished by using a simple linter that has the condition: `if nested component, and nested component has logic, then require id`. That might look something like

```html
<import id="list-item" src="./list-item.pc">
  <ul>
    {#each items as item}
    <list-item item="{item}" />
    <!--
     ^^^^^^^^^^^^^^^^^^^^^^
     This element needs an ID since it contains a logic controller.
     
    -->
    {/}
  </ul></import
>
```

So, attaching an ID to our list-item might look something like:

```html
<import id="list-item" src="./list-item.pc">
  <ul>
    {#each items as item}
    <list-item id="item-list-item" item="{item}" />
    {/}
  </ul></import
>
```

This aproach seems a bit clunky since `list-item` is already named. And actually `id="list-item"` already exists on the import statement, so we could simply use that id for all instances of `list-item`. For example:

```html
<import id="list-item" src="./list-item.pc">
  <logic src="./list.tsx">
    <ul>
      {#each items as item}
      <list-item item="{item}" />
      {/}
    </ul></logic
  ></import
>
```

And for our logic file:

```typescript
import {React} from "react";
import {BaseProps} from "./list.pc";
import {Item} from "./models";

type State = {
  items: Item[]
};

// No exposed props since List has everything it needs
export type Props = {};

export const (BaseList: React.Factory<BaseProps>) => (props: Props) => {
  const [state, setState] = useState({
    items: [
      { id: 1, label: "Clean dishes" },
      { id: 2, label: "Eat" },
    ]
  });

  const onChange = (newItem) => setState({
    ...state,
    items: state.items.map(item => item.id === newItem.id ? newItem : item)
  });

  return <BaseList items={items} listItemProps={{
    onChange
  }}>
};
```

This seems like a decent pattern. If people need specificity, then they can describe their components a bit more:

```html
<import id="button" src="./button.pc">
  <button id="sign-up-button">Sign Up</button>
  <button id="log-in-button">Log In</button></import
>
```

People could even use import the same component for the same effect:

````html
```html
<import id="button" src="./button.pc">
  <import id="sign-up-button" src="./button.pc">
    <sign-up-button>Sign Up</sign-up-button>
    <sign-up-button>Sign Up</sign-up-button>

    <!-- all log-in-button instances receive the same props -->
    <log-in-button>Log In</log-in-button>
    <log-in-button>Log In</log-in-button>
    <log-in-button>Log In</log-in-button>
    <log-in-button>Log In</log-in-button></import
  ></import
>
````

Jumping back a bit though, the logic code above has a problem: what happens if a parent `*.pc` component of list uses it? In the template, `items` is a required props, but not in logic part of it. For example:

```html
<import id="list" src="./list.pc">
  <list items={[{ label: "Clean dishes" }, { label: "Wash car" }]}></import
>
```

☝🏻If we compile this as-is, there would be a no-op. Maybe that's okay though since template props are specific to UI only. I could imagine a scenario however where variants of components are used:

```html
<import id="button" src="./button.pc">
  <button dark/>
  <button light
/></import>
```

So I think it will be difficult to say: "treat all view props as no-op". We'll want some escape hatch to ensure that template props pass through. Here's one possible option:

```typescript
export default (BaseButton: React.Factory<BaseProps>) => (props: Props) => {
  return <BaseButton {...props} />;
};
```

I find this pattern to be terribly opaque. Is there any way to pass the props behind the scenes? That could possibly be done with hooks:

```typescript
export const useLogic = (props: Props) => {
  // return props here
};
```

Then in the compiled code:

```typescript
import {useLogic, Props} from "./button.jsx";

type TemplateProps = {
  dark: boolean,
  light: boolean
};

export default (props: Props & ViewProps) => {
  const internalProps = useLogic(props);
  const {dark, light} = props;

  // for attribute variants
  return <div {{dark, light}}>
    {internalProps}
  </div>
}
```

But what about render props, or overriding children? You can't shove react children in returned hook props. Another option might be to
lean on tye safety. For example:

````typescript
import {Props as ViewProps} from "./button.jsx";

export type Props = {} & ViewProps;

export const (View: React.Factory<Props>) => (props: Props) => {
  return <View {...props} />;
};
```
````

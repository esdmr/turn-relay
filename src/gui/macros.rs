macro_rules! router_component {
	(
		message enum $Message:ident {
			$($message:tt)*
		}

		state enum $State:ident {
			$($state_name:ident ( $state_type:ty )),* $(,)?
		}

		$(impl Default for $default_state:ident;)?

		type ExtraUpdateArgs<$ExtraUpdateArgsL:lifetime> = $ExtraUpdateArgs:ty;
		type ExtraViewArgs<$ExtraViewArgsL:lifetime> = $ExtraViewArgs:ty;
		type ExtraSubscriptionArgs<$ExtraSubscriptionArgsL:lifetime> = $ExtraSubscriptionArgs:ty;

		fn update(..) {
			$($items:tt)*
		}
	) => {
		#[derive(Debug, Clone)]
		pub enum $Message {
			$($state_name ( <$state_type as $crate::gui::types::IcedComponent>::Message ),)*
			$($message)*
		}

		$(impl From<<$state_type as $crate::gui::types::IcedComponent>::Message> for $Message {
			fn from(value: <$state_type as $crate::gui::types::IcedComponent>::Message) -> Self {
				Self::$state_name(value)
			}
		})*

		$(impl Default for $State {
			fn default() -> Self {
				Self::$default_state(Default::default())
			}
		})?

		#[derive(Debug, Clone)]
		pub enum $State {
			Intermediate,
			$($state_name ( $state_type ),)*
		}

		impl $crate::gui::types::IcedComponent for $State {
			type Message = $Message;
			type TaskMessage = $Message;
			type ExtraUpdateArgs<$ExtraUpdateArgsL> = $ExtraUpdateArgs;
			type ExtraViewArgs<$ExtraViewArgsL> = $ExtraViewArgs;
			type ExtraSubscriptionArgs<$ExtraSubscriptionArgsL> = $ExtraSubscriptionArgs;

			fn update(
				&mut self,
				message: Self::Message,
				extra: Self::ExtraUpdateArgs<'_>,
			) -> ::iced::Task<Self::TaskMessage> {
				router_component! {
					_define_update
					self message extra
					{$($items)*}
					{$($state_name ( $state_type ),)*}
					{}
				}
			}

			fn view<'a>(&'a self, extra: Self::ExtraViewArgs<'_>) -> ::iced::Element<'a, Self::Message> {
				match self {
					$(Self::$state_name(i) => i.view(extra).map(Self::Message::$state_name),)*

					Self::Intermediate => {
						unreachable!("Fatal: UI state is in an intermediate state");
					}
				}
			}
		}
	};

	(
		_define_update
		$self:ident $message:ident $extra:ident
		{}
		{$($name:ident ( $_:ty )),* $(,)?}
		{$($cases:tt)*}
	) => {
		match (::std::mem::replace($self, Self::Intermediate), $message) {
			$($cases)*

			$((Self::$name(mut i), Self::Message::$name(message)) => {
				let task = i.update(message, $extra);
				*$self = Self::$name(i);
				return task;
			}

			(state, Self::Message::$name(message)) => {
				unreachable!("Invalid message: {:?} @ {state:?}", Self::Message::$name(message));
			})*

            (Self::Intermediate, _) => {
                unreachable!("Fatal: UI state is in an intermediate state");
            }
		}

		::iced::Task::none()
	};

	(
		_define_update
		$self:ident $message:ident $extra:ident
		{
			given $($Message:ident)|+ turn $OldState:ident into $NewState:ident;
			$($items:tt)*
		}
		{$($states:tt)*}
		{$($cases:tt)*}
	) => {
		router_component! {
			_define_update
			$self $message $extra
			{$($items)*}
			{$($states)*}
			{
				$($cases)*
				(
					Self::$OldState(i),
					$(Self::Message::$Message {..})|+,
				) => {
					*$self = Self::$NewState(i.into());
				}
			}
		}
	};

	(
		_define_update
		$self:ident $message:ident $extra:ident
		{
			given $($Message:ident $MessageA:tt)|+ turn $($OldState:ident($OldStateA:pat))|+ into $NewState:ident($NewStateA:expr) $(then $(($extra_args:pat_param))? $action:block)?;
			$($items:tt)*
		}
		{$($states:tt)*}
		{$($cases:tt)*}
	) => {
		router_component! {
			_define_update
			$self $message $extra
			{$($items)*}
			{$($states)*}
			{
				$($cases)*
				(
					$(Self::$OldState($OldStateA))|+,
					$(Self::Message::$Message $MessageA)|+,
				) => {
					*$self = Self::$NewState($NewStateA);
					$($(let $extra_args = $extra;)?
					$action)?
				}
			}
		}
	};

	(
		_define_update
		$self:ident $message:ident $extra:ident
		{
			given $($Message:ident $MessageA:tt)|+ pass $State:ident($new_message:expr);
			$($items:tt)*
		}
		{$($states:tt)*}
		{$($cases:tt)*}
	) => {
		router_component! {
			_define_update
			$self $message $extra
			{$($items)*}
			{$($states)*}
			{
				$($cases)*
				(
					Self::$State(mut i),
					$(Self::Message::$Message $MessageA)|+,
				) => {
					let task = i.update($new_message, $extra);
					*$self = Self::$State(i);
					return task;
				}
			}
		}
	};
	(
		_define_update
		$self:ident $message:ident $extra:ident
		{
			given $Message:ident ignore $State:ident;
			$($items:tt)*
		}
		{$($states:tt)*}
		{$($cases:tt)*}
	) => {
		router_component! {
			_define_update
			$self $message $extra
			{$($items)*}
			{$($states)*}
			{
				$($cases)*
				(
					Self::$State(i),
					Self::Message::$Message {..},
				) => {
					*$self = Self::$State(i);
					eprintln!("Warning: Ignoring message: {:?} @ {:?}", stringify!($Message), $self);
				}
			}
		}
	};
}

pub(crate) use router_component;

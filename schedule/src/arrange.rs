
// # TODO
// Volume calculation for the truck loading.  

// # REQUIREMENTS
// 1. Given a floor area and height need to find the maximal way to stack items from sequential orders in the space 
//      provided
// 2. The ability to stack items ontop of each other
// 3. The ability to stack different items ontop of each other
// 4. The ability to handle irregular shaped objects?
// 5. The ability to rotate objects on axis to maximise space.
// 6. Orientation of objects is important, ie bar leaners stacking ontop of each other.
// 7. Position of loading is important relative to delivery order.

// # QUESTIONS
// 1. Is there anywhere I can pull the dimensions of existing items?  Does Current have the capacity to store these? 
// 2. Since I will need to determine a relationship around stacking does it make sense to define the object dimensions
//      also in this application?
// 3. How should I store that relationship?  SqlLite appears to be a good option for this, but open to some other
//      nosql option or any thing else.
// 4. Need to define a relationship around what can stack on other objects.
// 5. Is there a way to visualise this?  WebGL, Some text representation.